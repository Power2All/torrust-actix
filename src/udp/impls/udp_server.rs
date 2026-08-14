use crate::config::enums::udp_receive_method::UdpReceiveMethod;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::announce_query_request::AnnounceQueryRequest;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_id::UserId;
use crate::udp::enums::request::Request;
use crate::udp::enums::response::Response;
use crate::udp::enums::server_error::ServerError;
use crate::udp::enums::udp_reply::UdpReply;
use crate::udp::structs::announce_interval::AnnounceInterval;
use crate::udp::structs::announce_request::AnnounceRequest;
use crate::udp::structs::announce_response::AnnounceResponse;
use crate::udp::structs::connect_request::ConnectRequest;
use crate::udp::structs::connect_response::ConnectResponse;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::error_response::ErrorResponse;
use crate::udp::structs::number_of_downloads::NumberOfDownloads;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::port::Port;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::scrape_request::ScrapeRequest;
use crate::udp::structs::scrape_response::ScrapeResponse;
use crate::udp::structs::torrent_scrape_statistics::TorrentScrapeStatistics;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::MAX_SCRAPE_TORRENTS;
use log::{
    debug,
    info
};
use smallvec::SmallVec;
use socket2::{
    Domain,
    Protocol,
    Socket,
    Type
};
use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr
};
use std::sync::{
    Arc,
    LazyLock
};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::runtime::Builder;

/// Lifetime of a UDP connection id in seconds. BEP 15 suggests two minutes.
const CONNECTION_ID_WINDOW_SECS: u64 = 120;

/// Per-process HMAC-SHA256 key for connection ids. Regenerated on every start, so ids do not
/// survive a restart.
///
/// This must be a keyed MAC, not a fast hash: `ahash` and friends are built for hash-map
/// distribution and make no key-recovery guarantee, and recovering the key would let an
/// attacker mint ids for addresses they do not control, restoring the reflection attack the
/// connection id exists to prevent.
static CONNECTION_ID_KEY: LazyLock<ring::hmac::Key> = LazyLock::new(|| {
    ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &ring::rand::SystemRandom::new())
        .expect("failed to generate the UDP connection-id key from the system RNG")
});

/// Byte range of the 40-character hex announce key inside `/announce/<key>/<user key>`.
const KEY_PATH_RANGE: std::ops::Range<usize> = 10..50;
/// Byte range of the 40-character hex user key inside `/announce/<key>/<user key>`.
const USER_KEY_PATH_RANGE: std::ops::Range<usize> = 51..91;

impl UdpServer {
    /// Binds the UDP tracker sockets for `bind_address` and prepares the configured receive
    /// backend (blocking recv, `recvmmsg`, `io_uring` or Windows RIO).
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the socket cannot be bound or configured.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(tracker: Arc<TorrentTracker>, bind_address: SocketAddr, udp_threads: usize, worker_threads: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool, use_payload_ip: bool, simple_proxy_protocol: bool, proxy_addrs: Arc<Vec<std::net::IpAddr>>, receive_method: UdpReceiveMethod) -> tokio::io::Result<UdpServer>
    {
        #[cfg(windows)]
        let use_rio = receive_method == UdpReceiveMethod::rio && {
            let available = crate::udp::impls::rio_recv::is_available();
            if !available {
                log::warn!("[UDP] RIO requested but unavailable on this system; falling back to standard receive");
            }
            available
        };
        #[cfg(not(windows))]
        let use_rio = false;

        let sockets = if use_rio {
            Vec::new()
        } else {
            Self::build_sockets(bind_address, udp_threads, recv_buffer_size, send_buffer_size, reuse_address)?
        };
        Ok(UdpServer {
            sockets,
            bind_address,
            recv_buffer_size,
            send_buffer_size,
            reuse_address,
            udp_threads,
            worker_threads,
            tracker,
            use_payload_ip,
            simple_proxy_protocol,
            proxy_addrs,
            receive_method,
        })
    }

    #[cfg(target_os = "linux")]
    fn build_sockets(bind_address: SocketAddr, count: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool) -> tokio::io::Result<Vec<Arc<UdpSocket>>> {
        let count = count.max(1);
        let mut sockets = Vec::with_capacity(count);
        for _ in 0..count {
            let socket = Self::configure_socket(bind_address, recv_buffer_size, send_buffer_size, reuse_address, true)?;
            sockets.push(Arc::new(socket));
        }
        Ok(sockets)
    }

    #[cfg(not(target_os = "linux"))]
    fn build_sockets(bind_address: SocketAddr, _count: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool) -> tokio::io::Result<Vec<Arc<UdpSocket>>> {
        let socket = Self::configure_socket(bind_address, recv_buffer_size, send_buffer_size, reuse_address, false)?;
        Ok(vec![Arc::new(socket)])
    }

    fn configure_socket(bind_address: SocketAddr, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool, reuse_port: bool) -> tokio::io::Result<UdpSocket> {
        let domain = if bind_address.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_recv_buffer_size(recv_buffer_size).map_err(tokio::io::Error::other)?;
        socket.set_send_buffer_size(send_buffer_size).map_err(tokio::io::Error::other)?;
        socket.set_reuse_address(reuse_address).map_err(tokio::io::Error::other)?;
        #[cfg(target_os = "linux")]
        if reuse_port {
            socket.set_reuse_port(true).map_err(tokio::io::Error::other)?;
        }
        #[cfg(not(target_os = "linux"))]
        let _ = reuse_port;
        socket.bind(&bind_address.into()).map_err(tokio::io::Error::other)?;
        socket.set_nonblocking(true).map_err(tokio::io::Error::other)?;
        let std_socket: std::net::UdpSocket = socket.into();
        UdpSocket::from_std(std_socket)
    }

    /// Runs the UDP receive/parse/respond loops until the shutdown watch channel fires.
    pub async fn start(&self, mut rx: tokio::sync::watch::Receiver<bool>) {
        let parse_pool = Arc::new(ParsePool::new(1_000_000, self.worker_threads));
        parse_pool.start_thread(self.worker_threads, self.tracker.clone(), rx.clone(), self.use_payload_ip, self.simple_proxy_protocol, self.proxy_addrs.clone()).await;
        let payload = parse_pool.payload.clone();
        let tracker_queue = self.tracker.clone();
        let mut rx_queue = rx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                    _ = rx_queue.changed() => {
                        break;
                    }
                    _ = interval.tick() => {
                        let len = payload.len() as i64;
                        tracker_queue.set_stats(StatsEvent::UdpQueueLen, len);
                    }
                }
            }
        });
        let udp_threads = self.udp_threads;
        let sockets = self.sockets.clone();
        let parse_pool_clone = parse_pool.clone();
        let receive_method = self.receive_method;
        let bind_address = self.bind_address;
        let recv_buffer_size = self.recv_buffer_size;
        let send_buffer_size = self.send_buffer_size;
        let reuse_address = self.reuse_address;
        tokio::task::spawn_blocking(move || {
            let tokio_udp = Builder::new_multi_thread()
                .thread_name("udp")
                .worker_threads(udp_threads)
                .enable_all()
                .build()
                .unwrap();
            tokio_udp.block_on(async move {
                #[cfg(windows)]
                if sockets.is_empty() {
                    info!("[UDP] receive backend: rio");
                    let parse_pool_rio = parse_pool_clone.clone();
                    let rx_rio = rx.clone();
                    match std::thread::Builder::new()
                        .name("udp-rio".to_string())
                        .spawn(move || {
                            crate::udp::impls::rio_recv::run(bind_address, recv_buffer_size, send_buffer_size, reuse_address, parse_pool_rio, rx_rio);
                        }) {
                            Ok(_handle) => {
                                rx.changed().await.ok();
                            }
                            Err(e) => {
                                log::error!("[UDP] failed to spawn RIO receive thread: {e}");
                                return;
                            }
                        }
                    return;
                }
                let _ = (bind_address, recv_buffer_size, send_buffer_size, reuse_address);

                #[cfg(target_os = "linux")]
                let use_io_uring = {
                    let requested = receive_method == UdpReceiveMethod::io_uring;
                    let available = requested && crate::udp::impls::io_uring_recv::is_available();
                    if requested && !available {
                        log::warn!("[UDP] io_uring requested but unavailable (kernel/seccomp); falling back to recvmmsg");
                    }
                    info!("[UDP] receive backend: {}", if available { "io_uring" } else { "recvmmsg" });
                    available
                };
                #[cfg(not(target_os = "linux"))]
                let _ = receive_method;

                for index in 0..udp_threads {
                    let parse_pool_clone = parse_pool_clone.clone();
                    let socket = sockets[index % sockets.len()].clone();
                    let rx = rx.clone();

                    #[cfg(target_os = "linux")]
                    if use_io_uring {
                        std::thread::Builder::new()
                            .name(format!("udp-uring-{index}"))
                            .spawn(move || {
                                crate::udp::impls::io_uring_recv::run(socket, parse_pool_clone, rx);
                            })
                            .expect("failed to spawn io_uring receive thread");
                    } else {
                        tokio::spawn(async move {
                            Self::recv_loop_recvmmsg(socket, parse_pool_clone, rx).await;
                        });
                    }

                    #[cfg(not(target_os = "linux"))]
                    tokio::spawn(async move {
                        Self::recv_loop(socket, parse_pool_clone, rx).await;
                    });
                }
                rx.changed().await.ok();
            });
        });
    }

    #[cfg(not(target_os = "linux"))]
    async fn recv_loop(socket: Arc<UdpSocket>, parse_pool: Arc<ParsePool>, mut rx: tokio::sync::watch::Receiver<bool>) {
        let udp_sock = socket.local_addr().unwrap();
        let mut data = [0u8; crate::udp::udp::MAX_PACKET_SIZE];
        loop {
            tokio::select! {
                _ = rx.changed() => {
                    info!("Stopping UDP server: {udp_sock}...");
                    break;
                }
                Ok((valid_bytes, remote_addr)) = socket.recv_from(&mut data) => {
                    if valid_bytes > 0 {
                        let packet = UdpPacket {
                            remote_addr,
                            data: SmallVec::from_slice(&data[..valid_bytes]),
                            reply: UdpReply::Socket(socket.clone()),
                        };
                        if parse_pool.payload.push(packet).is_err() {
                            debug!("Parse pool queue full, dropping packet");
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    async fn recv_loop_recvmmsg(socket: Arc<UdpSocket>, parse_pool: Arc<ParsePool>, mut rx: tokio::sync::watch::Receiver<bool>) {
        use crate::udp::impls::batch_recv::{RecvBatch, BATCH};
        use std::os::unix::io::AsRawFd;
        use tokio::io::Interest;

        const MAX_DRAIN_ROUNDS: usize = 16;

        let udp_sock = socket.local_addr().unwrap();
        let fd = socket.as_raw_fd();
        let mut batch = RecvBatch::new();
        loop {
            tokio::select! {
                biased;
                _ = rx.changed() => {
                    info!("Stopping UDP server: {udp_sock}...");
                    break;
                }
                readable = socket.readable() => {
                    if readable.is_err() {
                        break;
                    }
                    let mut rounds = 0;
                    loop {
                        match socket.try_io(Interest::READABLE, || batch.recv(fd)) {
                            Ok(count) => {
                                for i in 0..count {
                                    if let Some((buf, remote_addr)) = batch.datagram(i) {
                                        if buf.is_empty() {
                                            continue;
                                        }
                                        let packet = UdpPacket {
                                            remote_addr,
                                            data: SmallVec::from_slice(buf),
                                            reply: UdpReply::Socket(socket.clone()),
                                        };
                                        if parse_pool.payload.push(packet).is_err() {
                                            debug!("Parse pool queue full, dropping packet");
                                        }
                                    }
                                }
                                rounds += 1;
                                if count < BATCH || rounds >= MAX_DRAIN_ROUNDS {
                                    break;
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(_) => break,
                        }
                    }
                }
            }
        }
    }

    /// Encodes a tracker [`Response`] and sends it to the client, logging failures.
    pub async fn send_response(tracker: Arc<TorrentTracker>, reply: UdpReply, remote_addr: SocketAddr, response: Response) {
        debug!("sending response to: {remote_addr:?}");
        let estimated_size = response.estimated_size();
        let mut buffer = Vec::with_capacity(estimated_size);
        match response.write(&mut buffer) {
            Ok(()) => {
                UdpServer::send_packet(reply, &remote_addr, &buffer).await;
            }
            Err(error) => {
                match remote_addr {
                    SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4InvalidRequest, 1); }
                    SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6InvalidRequest, 1); }
                }
                debug!("could not write response to bytes: {error}");
            }
        }
    }

    /// Sends a raw datagram to the client via the backend-specific reply channel.
    pub async fn send_packet(reply: UdpReply, remote_addr: &SocketAddr, payload: &[u8]) {
        match reply {
            UdpReply::Socket(socket) => {
                let _ = socket.send_to(payload, remote_addr).await;
            }
            #[cfg(windows)]
            UdpReply::Rio(sender) => {
                sender.send(*remote_addr, payload);
            }
        }
    }

    /// Derives the BEP 15 connection id for a client address in a given time window.
    ///
    /// A keyed hash of `(window, port, ip)`, so a client cannot forge one and ids expire with
    /// the window without the tracker keeping per-client state.
    fn derive_connection_id(remote_address: &SocketAddr, window: u64) -> ConnectionId {
        let mut context = ring::hmac::Context::with_key(&CONNECTION_ID_KEY);
        context.update(&window.to_be_bytes());
        context.update(&remote_address.port().to_be_bytes());
        match remote_address.ip() {
            std::net::IpAddr::V4(ipv4) => context.update(&ipv4.octets()),
            std::net::IpAddr::V6(ipv6) => context.update(&ipv6.octets()),
        }
        let tag = context.sign();
        let truncated: [u8; 8] = tag.as_ref()[..8].try_into().expect("HMAC-SHA256 tag is 32 bytes");
        ConnectionId(i64::from_be_bytes(truncated))
    }

    /// Returns the current time window index used to derive connection ids.
    #[inline]
    fn connection_id_window() -> u64 {
        crate::common::common::current_time() / CONNECTION_ID_WINDOW_SECS
    }

    /// Derives the BEP 15 connection id to hand a client in response to a connect request.
    pub async fn get_connection_id(remote_address: &SocketAddr) -> ConnectionId {
        Self::derive_connection_id(remote_address, Self::connection_id_window())
    }

    /// Checks a connection id supplied in an announce or scrape request against the ids this
    /// tracker would have issued to `remote_address`.
    ///
    /// Required by BEP 15: without it the tracker answers announces from forged source
    /// addresses, acting as a UDP reflector.
    #[inline]
    pub fn connection_id_valid(remote_address: &SocketAddr, connection_id: ConnectionId) -> bool {
        let window = Self::connection_id_window();
        // The previous window is accepted so an id stays usable across a window boundary.
        connection_id == Self::derive_connection_id(remote_address, window)
            || connection_id == Self::derive_connection_id(remote_address, window.wrapping_sub(1))
    }

    /// Parses one datagram and produces the tracker response, mapping malformed input and
    /// handler failures to BEP 15 error responses.
    pub async fn handle_packet(remote_addr: SocketAddr, payload: &[u8], tracker: Arc<TorrentTracker>, use_payload_ip: bool) -> Response {
        if payload.len() == 16 && let [_, _, _, _, action1, action2, action3, action4, ..] = payload && *action1 == 0 && *action2 == 0 && *action3 == 0 && *action4 == 0 && let Ok(Request::Connect(connect_request)) = Request::from_bytes(payload, MAX_SCRAPE_TORRENTS) {
            return match UdpServer::handle_udp_connect(remote_addr, &connect_request, tracker).await {
                Ok(response) => response,
                Err(e) => UdpServer::handle_udp_error(e, connect_request.transaction_id).await,
            }
        }
        let transaction_id = if let Ok(request) = Request::from_bytes(payload, MAX_SCRAPE_TORRENTS) {
            let tid = match &request {
                Request::Connect(connect_request) => connect_request.transaction_id,
                Request::Announce(announce_request) => announce_request.transaction_id,
                Request::Scrape(scrape_request) => scrape_request.transaction_id,
            };
            match UdpServer::handle_request(request, remote_addr, tracker.clone(), use_payload_ip).await {
                Ok(response) => return response,
                Err(_e) => {
                    match remote_addr {
                        SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4InvalidRequest, 1); }
                        SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6InvalidRequest, 1); }
                    }
                    tid
                }
            }
        } else {
            match remote_addr {
                SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4BadRequest, 1); }
                SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6BadRequest, 1); }
            }
            TransactionId(0)
        };
        UdpServer::handle_udp_error(ServerError::BadRequest, transaction_id).await
    }

    /// Dispatches a parsed request to the connect, announce or scrape handler after updating
    /// the request statistics.
    ///
    /// # Errors
    ///
    /// Returns a [`ServerError`] describing why the request was refused.
    pub async fn handle_request(request: Request, remote_addr: SocketAddr, tracker: Arc<TorrentTracker>, use_payload_ip: bool) -> Result<Response, ServerError> {
        // Gated like every other instrumented site: the tag values below allocate five Strings
        // per datagram (one of them a hex encode of the info-hash), which is not something the
        // UDP hot path should pay for when nothing is collecting traces.
        let transaction_guard = crate::utils::sentry_tracing::start_trace_transaction("udp server", "handle packet");
        let result = match &request {
            Request::Connect(connect_request) => {
                UdpServer::handle_udp_connect(remote_addr, connect_request, tracker).await
            }
            Request::Announce(announce_request) => {
                UdpServer::handle_udp_announce(remote_addr, announce_request, tracker, use_payload_ip).await
            }
            Request::Scrape(scrape_request) => {
                UdpServer::handle_udp_scrape(remote_addr, scrape_request, tracker).await
            }
        };
        if let Some(transaction_guard) = transaction_guard {
            match &request {
                Request::Connect(_) => {
                    transaction_guard.set_tag("request_type", "connect");
                }
                Request::Announce(announce_request) => {
                    transaction_guard.set_tag("request_type", "announce");
                    transaction_guard.set_tag("info_hash", hex::encode(announce_request.info_hash.0));
                }
                Request::Scrape(scrape_request) => {
                    transaction_guard.set_tag("request_type", "scrape");
                    transaction_guard.set_tag("num_info_hashes", scrape_request.info_hashes.len().to_string());
                }
            }
            transaction_guard.set_tag("remote_addr", remote_addr.to_string());
            transaction_guard.set_tag("use_payload_ip", use_payload_ip.to_string());
            match &result {
                Ok(_) => transaction_guard.set_tag("result", "success"),
                Err(e) => transaction_guard.set_tag("result", format!("error: {e:?}")),
            }
            transaction_guard.finish();
        }
        result
    }

    /// Handles a BEP 15 connect request: returns the derived connection id.
    ///
    /// # Errors
    ///
    /// Currently infallible; kept as `Result` for interface symmetry.
    pub async fn handle_udp_connect(remote_addr: SocketAddr, request: &ConnectRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        let connection_id = UdpServer::get_connection_id(&remote_addr).await;
        let response = Response::from(ConnectResponse {
            transaction_id: request.transaction_id,
            connection_id
        });
        let stats_event = if remote_addr.is_ipv4() {
            StatsEvent::Udp4ConnectionsHandled
        } else {
            StatsEvent::Udp6ConnectionsHandled
        };
        tracker.update_stats(stats_event, 1);
        Ok(response)
    }

    /// Handles a BEP 15 announce: enforces whitelist/blacklist/keys/users (keys travel in the
    /// request path), updates the swarm and returns up to 72 packed peers of the client's IP family.
    ///
    /// # Errors
    ///
    /// Returns a [`ServerError`] when access rules reject the request or the swarm update fails.
    pub async fn handle_udp_announce(remote_addr: SocketAddr, request: &AnnounceRequest, tracker: Arc<TorrentTracker>, use_payload_ip: bool) -> Result<Response, ServerError> {
        if !Self::connection_id_valid(&remote_addr, request.connection_id) {
            debug!("[UDP ERROR] Invalid connection id from {remote_addr}");
            return Err(ServerError::InvalidConnectionId);
        }
        let config = &tracker.config.tracker_config;
        let effective_remote_addr = if use_payload_ip {
            if let Some(payload_ip) = request.ip_address {
                SocketAddr::new(std::net::IpAddr::V4(payload_ip), remote_addr.port())
            } else {
                remote_addr
            }
        } else {
            remote_addr
        };
        if config.whitelist_enabled && !tracker.check_whitelist(InfoHash(request.info_hash.0)) {
            debug!("[UDP ERROR] Torrent Not Whitelisted");
            return Err(ServerError::TorrentNotWhitelisted);
        }
        if config.blacklist_enabled && tracker.check_blacklist(InfoHash(request.info_hash.0)) {
            debug!("[UDP ERROR] Torrent Blacklisted");
            return Err(ServerError::TorrentBlacklisted);
        }
        // Slice the bytes, not the String: `path` is arbitrary client-supplied UTF-8, and a
        // String slice at a byte offset inside a multi-byte character panics.
        if config.keys_enabled {
            let Some(key_hex) = request.path.as_bytes().get(KEY_PATH_RANGE) else {
                debug!("[UDP ERROR] Unknown Key - path too short");
                return Err(ServerError::UnknownKey);
            };
            match hex::decode(key_hex) {
                Ok(result) if result.len() >= 20 => {
                    let key: [u8; 20] = result[..20].try_into().expect("length checked above");
                    if !tracker.check_key(InfoHash::from(key)) {
                        debug!("[UDP ERROR] Unknown Key");
                        return Err(ServerError::UnknownKey);
                    }
                }
                _ => {
                    debug!("[UDP ERROR] Unknown Key - not valid hex");
                    return Err(ServerError::UnknownKey);
                }
            }
        }
        let user_key = if config.users_enabled {
            let Some(user_key_hex) = request.path.as_bytes().get(USER_KEY_PATH_RANGE) else {
                debug!("[UDP ERROR] Peer Key Not Valid - path too short");
                return Err(ServerError::PeerKeyNotValid);
            };
            match hex::decode(user_key_hex) {
                Ok(result) if result.len() >= 20 => {
                    let key: [u8; 20] = result[..20].try_into().expect("length checked above");
                    tracker.check_user_key(UserId::from(key))
                }
                _ => {
                    debug!("[UDP ERROR] Peer Key Not Valid");
                    return Err(ServerError::PeerKeyNotValid);
                }
            }
        } else {
            None
        };
        if config.users_enabled && user_key.is_none() {
            debug!("[UDP ERROR] Peer Key Not Valid");
            return Err(ServerError::PeerKeyNotValid);
        }
        // BEP 15 lets a client send -1 for "tracker's choice"; casting that straight to u64 would
        // land at u64::MAX, so it goes through the same 1..=72 clamp as the HTTP path. Held in a
        // local because the response below is sized and truncated by it.
        let want = match request.peers_wanted.0 {
            wanted @ 1..=72 => wanted as usize,
            _ => 72,
        };
        let announce_request = AnnounceQueryRequest {
            info_hash: InfoHash(request.info_hash.0),
            peer_id: PeerId(request.peer_id.0),
            port: request.port.0,
            uploaded: request.bytes_uploaded.0 as u64,
            downloaded: request.bytes_downloaded.0 as u64,
            left: request.bytes_left.0 as u64,
            compact: false,
            no_peer_id: false,
            event: request.event,
            remote_addr: effective_remote_addr.ip(),
            numwant: want as u64,
            rtctorrent: None,
            rtcoffer: None,
            rtcrequest: None,
            rtcanswer: None,
            rtcanswerfor: None,
        };
        let torrent = match tracker.handle_announce(&announce_request, user_key).await {
            Ok(result) => result,
            Err(error) => {
                debug!("[UDP ERROR] Handle Announce - Internal Server Error: {error:#?}");
                return Err(ServerError::InternalServerError);
            }
        };
        let self_peer_id = PeerId(request.peer_id.0);
        let mut peers: Vec<ResponsePeer<Ipv4Addr>> = Vec::with_capacity(want);
        let mut peers6: Vec<ResponsePeer<Ipv6Addr>> = Vec::with_capacity(want);
        if request.bytes_left.0 != 0 {
            if effective_remote_addr.is_ipv4() {
                for (peer_id, torrent_peer) in &torrent.seeds {
                    if peers.len() >= want { break; }
                    if *peer_id == self_peer_id { continue; }
                    if let std::net::IpAddr::V4(ip) = torrent_peer.peer_addr.ip() {
                        peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            } else {
                for (peer_id, torrent_peer) in &torrent.seeds_ipv6 {
                    if peers6.len() >= want { break; }
                    if *peer_id == self_peer_id { continue; }
                    if let std::net::IpAddr::V6(ip) = torrent_peer.peer_addr.ip() {
                        peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            }
        }
        if effective_remote_addr.is_ipv4() {
            for (peer_id, torrent_peer) in &torrent.peers {
                if peers.len() >= want { break; }
                if *peer_id == self_peer_id { continue; }
                if let std::net::IpAddr::V4(ip) = torrent_peer.peer_addr.ip() {
                    peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                }
            }
        } else {
            for (peer_id, torrent_peer) in &torrent.peers_ipv6 {
                if peers6.len() >= want { break; }
                if *peer_id == self_peer_id { continue; }
                if let std::net::IpAddr::V6(ip) = torrent_peer.peer_addr.ip() {
                    peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                }
            }
        }
        let request_interval = config.request_interval as i32;
        let leechers = torrent.counts.total_peers() as i32;
        let seeders = torrent.counts.total_seeds() as i32;
        let response = if effective_remote_addr.is_ipv6() {
            Response::from(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(request_interval),
                leechers: NumberOfPeers(leechers),
                seeders: NumberOfPeers(seeders),
                peers: peers6,
            })
        } else {
            Response::from(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(request_interval),
                leechers: NumberOfPeers(leechers),
                seeders: NumberOfPeers(seeders),
                peers,
            })
        };
        let stats_event = if remote_addr.is_ipv4() {
            StatsEvent::Udp4AnnouncesHandled
        } else {
            StatsEvent::Udp6AnnouncesHandled
        };
        tracker.update_stats(stats_event, 1);
        Ok(response)
    }

    /// Handles a BEP 15 scrape: returns seeders/completed/leechers for each requested info-hash
    /// (zeroes for unknown torrents).
    ///
    /// # Errors
    ///
    /// Currently infallible; kept as `Result` for interface symmetry.
    pub async fn handle_udp_scrape(remote_addr: SocketAddr, request: &ScrapeRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        if !Self::connection_id_valid(&remote_addr, request.connection_id) {
            debug!("[UDP ERROR] Invalid connection id from {remote_addr}");
            return Err(ServerError::InvalidConnectionId);
        }
        let mut torrent_stats = Vec::with_capacity(request.info_hashes.len());
        for info_hash in &request.info_hashes {
            let scrape_entry = match tracker.get_torrent_counts(InfoHash(info_hash.0)) {
                Some(counts) => TorrentScrapeStatistics {
                    seeders: NumberOfPeers(counts.total_seeds() as i32),
                    completed: NumberOfDownloads(counts.completed as i32),
                    leechers: NumberOfPeers(counts.total_peers() as i32),
                },
                None => TorrentScrapeStatistics {
                    seeders: NumberOfPeers(0),
                    completed: NumberOfDownloads(0),
                    leechers: NumberOfPeers(0),
                },
            };
            torrent_stats.push(scrape_entry);
        }
        let stats_event = if remote_addr.is_ipv4() {
            StatsEvent::Udp4ScrapesHandled
        } else {
            StatsEvent::Udp6ScrapesHandled
        };
        tracker.update_stats(stats_event, 1);
        Ok(Response::from(ScrapeResponse {
            transaction_id: request.transaction_id,
            torrent_stats,
        }))
    }

    /// Builds the BEP 15 error response for a failed request.
    pub async fn handle_udp_error(e: ServerError, transaction_id: TransactionId) -> Response {
        Response::from(ErrorResponse {
            transaction_id,
            message: e.to_string().into()
        })
    }
}