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
use crate::udp::structs::udp_packet::{
    UdpPacket,
    UdpReply
};
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
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::runtime::Builder;

impl UdpServer {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(tracker: Arc<TorrentTracker>, bind_address: SocketAddr, udp_threads: usize, worker_threads: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool, use_payload_ip: bool, simple_proxy_protocol: bool, receive_method: UdpReceiveMethod) -> tokio::io::Result<UdpServer>
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

    pub async fn start(&self, mut rx: tokio::sync::watch::Receiver<bool>) {
        let parse_pool = Arc::new(ParsePool::new(1_000_000, self.worker_threads));
        parse_pool.start_thread(self.worker_threads, self.tracker.clone(), rx.clone(), self.use_payload_ip, self.simple_proxy_protocol).await;
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
                    std::thread::Builder::new()
                        .name("udp-rio".to_string())
                        .spawn(move || {
                            crate::udp::impls::rio_recv::run(bind_address, recv_buffer_size, send_buffer_size, reuse_address, parse_pool_rio, rx_rio);
                        })
                        .expect("failed to spawn RIO receive thread");
                    rx.changed().await.ok();
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

    pub async fn send_response(tracker: Arc<TorrentTracker>, reply: UdpReply, remote_addr: SocketAddr, response: Response) {
        debug!("sending response to: {:?}", &remote_addr);
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

    pub async fn get_connection_id(remote_address: &SocketAddr) -> ConnectionId {
        use std::hash::{
            DefaultHasher,
            Hasher
        };
        use std::time::{
            SystemTime,
            UNIX_EPOCH
        };

        let mut hasher = DefaultHasher::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        hasher.write_u64(timestamp);
        hasher.write_u16(remote_address.port());
        if let std::net::IpAddr::V4(ipv4) = remote_address.ip() {
            hasher.write(&ipv4.octets());
        } else if let std::net::IpAddr::V6(ipv6) = remote_address.ip() {
            hasher.write(&ipv6.octets());
        }
        ConnectionId(hasher.finish() as i64)
    }

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

    pub async fn handle_request(request: Request, remote_addr: SocketAddr, tracker: Arc<TorrentTracker>, use_payload_ip: bool) -> Result<Response, ServerError> {
        let transaction = sentry::TransactionContext::new("udp server", "handle packet");
        let transaction_guard = sentry::start_transaction(transaction);
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
        result
    }

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

    pub async fn handle_udp_announce(remote_addr: SocketAddr, request: &AnnounceRequest, tracker: Arc<TorrentTracker>, use_payload_ip: bool) -> Result<Response, ServerError> {
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
        if config.keys_enabled {
            if request.path.len() < 50 {
                debug!("[UDP ERROR] Unknown Key");
                return Err(ServerError::UnknownKey);
            }
            let key_path_extract = &request.path[10..50];
            if let Ok(result) = hex::decode(key_path_extract) {
                if result.len() >= 20 {
                    let key = <[u8; 20]>::try_from(&result[0..20]).unwrap();
                    if !tracker.check_key(InfoHash::from(key)) {
                        debug!("[UDP ERROR] Unknown Key");
                        return Err(ServerError::UnknownKey);
                    }
                } else {
                    debug!("[UDP ERROR] Unknown Key - insufficient bytes");
                    return Err(ServerError::UnknownKey);
                }
            } else {
                debug!("[UDP ERROR] Unknown Key");
                return Err(ServerError::UnknownKey);
            }
        }
        let user_key = if config.users_enabled {
            let user_key_path_extract = if request.path.len() >= 91 {
                Some(&request.path[51..=91])
            } else if !config.users_enabled && request.path.len() >= 50 {
                Some(&request.path[10..=50])
            } else {
                None
            };
            if let Some(path) = user_key_path_extract {
                match hex::decode(path) {
                    Ok(result) if result.len() >= 20 => {
                        let key = <[u8; 20]>::try_from(&result[0..20]).unwrap();
                        tracker.check_user_key(UserId::from(key))
                    }
                    _ => {
                        debug!("[UDP ERROR] Peer Key Not Valid");
                        return Err(ServerError::PeerKeyNotValid);
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        if config.users_enabled && user_key.is_none() {
            debug!("[UDP ERROR] Peer Key Not Valid");
            return Err(ServerError::PeerKeyNotValid);
        }
        let torrent = match tracker.handle_announce(tracker.clone(), AnnounceQueryRequest {
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
            numwant: request.peers_wanted.0 as u64,
            rtctorrent: None,
            rtcoffer: None,
            rtcrequest: None,
            rtcanswer: None,
            rtcanswerfor: None,
        }, user_key).await {
            Ok(result) => result.1,
            Err(error) => {
                debug!("[UDP ERROR] Handle Announce - Internal Server Error: {error:#?}");
                return Err(ServerError::InternalServerError);
            }
        };
        let self_peer_id = PeerId(request.peer_id.0);
        let mut peers: Vec<ResponsePeer<Ipv4Addr>> = Vec::with_capacity(72);
        let mut peers6: Vec<ResponsePeer<Ipv6Addr>> = Vec::with_capacity(72);
        if request.bytes_left.0 != 0 {
            if effective_remote_addr.is_ipv4() {
                for (peer_id, torrent_peer) in &torrent.seeds {
                    if peers.len() >= 72 { break; }
                    if *peer_id == self_peer_id { continue; }
                    if let std::net::IpAddr::V4(ip) = torrent_peer.peer_addr.ip() {
                        peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            } else {
                for (peer_id, torrent_peer) in &torrent.seeds_ipv6 {
                    if peers6.len() >= 72 { break; }
                    if *peer_id == self_peer_id { continue; }
                    if let std::net::IpAddr::V6(ip) = torrent_peer.peer_addr.ip() {
                        peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            }
        }
        if effective_remote_addr.is_ipv4() {
            for (peer_id, torrent_peer) in &torrent.peers {
                if peers.len() >= 72 { break; }
                if *peer_id == self_peer_id { continue; }
                if let std::net::IpAddr::V4(ip) = torrent_peer.peer_addr.ip() {
                    peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                }
            }
        } else {
            for (peer_id, torrent_peer) in &torrent.peers_ipv6 {
                if peers6.len() >= 72 { break; }
                if *peer_id == self_peer_id { continue; }
                if let std::net::IpAddr::V6(ip) = torrent_peer.peer_addr.ip() {
                    peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                }
            }
        }
        let request_interval = config.request_interval as i32;
        let leechers = (torrent.peers.len() + torrent.peers_ipv6.len()) as i32;
        let seeders = (torrent.seeds.len() + torrent.seeds_ipv6.len()) as i32;
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

    pub async fn handle_udp_scrape(remote_addr: SocketAddr, request: &ScrapeRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
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

    pub async fn handle_udp_error(e: ServerError, transaction_id: TransactionId) -> Response {
        Response::from(ErrorResponse {
            transaction_id,
            message: e.to_string().into()
        })
    }
}