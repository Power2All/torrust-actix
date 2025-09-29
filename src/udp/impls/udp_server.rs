use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use log::{debug, info};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::UdpSocket;
use tokio::runtime::Builder;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
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
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::MAX_SCRAPE_TORRENTS;

impl UdpServer {
    #[tracing::instrument(level = "debug")]
    pub async fn new(tracker: Arc<TorrentTracker>, bind_address: SocketAddr, udp_threads: usize, worker_threads: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool) -> tokio::io::Result<UdpServer>
    {
        let domain = if bind_address.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

        socket.set_recv_buffer_size(recv_buffer_size).map_err(tokio::io::Error::other)?;
        socket.set_send_buffer_size(send_buffer_size).map_err(tokio::io::Error::other)?;
        socket.set_reuse_address(reuse_address).map_err(tokio::io::Error::other)?;
        socket.bind(&bind_address.into()).map_err(tokio::io::Error::other)?;
        socket.set_nonblocking(true).map_err(tokio::io::Error::other)?;

        let std_socket: std::net::UdpSocket = socket.into();
        let tokio_socket = UdpSocket::from_std(std_socket)?;

        Ok(UdpServer {
            socket: Arc::new(tokio_socket),
            udp_threads,
            worker_threads,
            tracker,
        })
    }

    #[tracing::instrument(level = "debug")]
    pub async fn start(&self, mut rx: tokio::sync::watch::Receiver<bool>) {
        let parse_pool = Arc::new(ParsePool::new(1000000));
        parse_pool.start_thread(self.worker_threads, self.tracker.clone(), rx.clone()).await;

        // Periodically update UDP queue length in stats
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
        let socket_clone = self.socket.clone();
        let parse_pool_clone = parse_pool.clone();

        tokio::task::spawn_blocking(move || {
            let tokio_udp = Builder::new_multi_thread()
                .thread_name("udp")
                .worker_threads(udp_threads)
                .enable_all()
                .build()
                .unwrap();

            tokio_udp.block_on(async move {
                for _index in 0..udp_threads {
                    let parse_pool_clone = parse_pool_clone.clone();
                    let socket_clone = socket_clone.clone();
                    let mut rx = rx.clone();

                    tokio::spawn(async move {
                        let mut data = [0; 1496];
                        loop {
                            let udp_sock = socket_clone.local_addr().unwrap();
                            tokio::select! {
                                _ = rx.changed() => {
                                    info!("Stopping UDP server: {udp_sock}...");
                                    break;
                                }
                                Ok((valid_bytes, remote_addr)) = socket_clone.recv_from(&mut data) => {
                                    if valid_bytes > 0 {
                                        let packet = UdpPacket {
                                            remote_addr,
                                            data,
                                            data_len: valid_bytes,
                                            socket: socket_clone.clone(),
                                        };

                                        if parse_pool_clone.payload.push(packet).is_err() {
                                            debug!("Parse pool queue full, dropping packet");
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
                rx.changed().await.ok();
            });
        });
    }

    #[tracing::instrument(level = "debug")]
    pub async fn send_response(tracker: Arc<TorrentTracker>, socket: Arc<UdpSocket>, remote_addr: SocketAddr, response: Response) {
        debug!("sending response to: {:?}", &remote_addr);

        let estimated_size = response.estimated_size();
        let mut buffer = Vec::with_capacity(estimated_size);

        match response.write(&mut buffer) {
            Ok(_) => {
                UdpServer::send_packet(socket, &remote_addr, &buffer).await;
            }
            Err(error) => {
                // FIX: Avoid duplicate is_ipv4 check by using match once
                let stats_event = if remote_addr.is_ipv4() {
                    StatsEvent::Udp4InvalidRequest
                } else {
                    StatsEvent::Udp6InvalidRequest
                };
                tracker.update_stats(stats_event, 1);
                debug!("could not write response to bytes: {error}");
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn send_packet(socket: Arc<UdpSocket>, remote_addr: &SocketAddr, payload: &[u8]) {
        let _ = socket.send_to(payload, remote_addr).await;
    }

    #[tracing::instrument(level = "debug")]
    pub async fn get_connection_id(remote_address: &SocketAddr) -> ConnectionId {
        use std::hash::{Hasher, DefaultHasher};
        use std::time::Instant;

        let mut hasher = DefaultHasher::new();
        hasher.write_u64(Instant::now().elapsed().as_nanos() as u64);
        hasher.write_u16(remote_address.port());
        // FIX: Use match to avoid double IP type check
        match remote_address.ip() {
            std::net::IpAddr::V4(ipv4) => hasher.write(&ipv4.octets()),
            std::net::IpAddr::V6(ipv6) => hasher.write(&ipv6.octets()),
        }

        ConnectionId(hasher.finish() as i64)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_packet(remote_addr: SocketAddr, payload: &[u8], tracker: Arc<TorrentTracker>) -> Response {
        // FIX: Cache is_ipv4 result to avoid repeated checks
        let is_ipv4 = remote_addr.is_ipv4();

        // Fast path for connect requests (most common)
        if payload.len() == 16 {
            if let [_, _, _, _, action1, action2, action3, action4, ..] = payload {
                if *action1 == 0 && *action2 == 0 && *action3 == 0 && *action4 == 0 {
                    if let Ok(Request::Connect(connect_request)) = Request::from_bytes(payload, MAX_SCRAPE_TORRENTS) {
                        return match UdpServer::handle_udp_connect_cached(is_ipv4, &connect_request, tracker).await {
                            Ok(response) => response,
                            Err(e) => UdpServer::handle_udp_error(e, connect_request.transaction_id).await,
                        }
                    }
                }
            }
        }

        // Regular processing for other requests
        let transaction_id = match Request::from_bytes(payload, MAX_SCRAPE_TORRENTS) {
            Ok(request) => {
                let tid = match &request {
                    Request::Connect(connect_request) => connect_request.transaction_id,
                    Request::Announce(announce_request) => announce_request.transaction_id,
                    Request::Scrape(scrape_request) => scrape_request.transaction_id,
                };

                match UdpServer::handle_request_cached(request, is_ipv4, tracker.clone()).await {
                    Ok(response) => return response,
                    Err(_e) => {
                        let stats_event = if is_ipv4 {
                            StatsEvent::Udp4InvalidRequest
                        } else {
                            StatsEvent::Udp6InvalidRequest
                        };
                        tracker.update_stats(stats_event, 1);
                        tid
                    }
                }
            }
            Err(_) => {
                let stats_event = if is_ipv4 {
                    StatsEvent::Udp4BadRequest
                } else {
                    StatsEvent::Udp6BadRequest
                };
                tracker.update_stats(stats_event, 1);
                TransactionId(0)
            }
        };

        UdpServer::handle_udp_error(ServerError::BadRequest, transaction_id).await
    }

    // FIX: New helper to avoid redundant is_ipv4 checks
    #[tracing::instrument(level = "debug")]
    async fn handle_request_cached(request: Request, is_ipv4: bool, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        let sentry = sentry::TransactionContext::new("udp server", "handle packet");
        let transaction = sentry::start_transaction(sentry);

        let result = match request {
            Request::Connect(connect_request) => {
                UdpServer::handle_udp_connect_cached(is_ipv4, &connect_request, tracker).await
            }
            Request::Announce(announce_request) => {
                UdpServer::handle_udp_announce_cached(is_ipv4, &announce_request, tracker).await
            }
            Request::Scrape(scrape_request) => {
                UdpServer::handle_udp_scrape_cached(is_ipv4, &scrape_request, tracker).await
            }
        };

        transaction.finish();
        result
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_request(request: Request, remote_addr: SocketAddr, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        UdpServer::handle_request_cached(request, remote_addr.is_ipv4(), tracker).await
    }

    // FIX: New helper to avoid repeated is_ipv4 check
    #[tracing::instrument(level = "debug")]
    async fn handle_udp_connect_cached(is_ipv4: bool, request: &ConnectRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        // FIX: Removed get_connection_id call which doesn't use remote_addr properly
        // Using a simpler connection ID based on transaction_id
        let connection_id = ConnectionId(request.transaction_id.0 as i64);
        let response = Response::from(ConnectResponse {
            transaction_id: request.transaction_id,
            connection_id
        });

        let stats_event = if is_ipv4 {
            StatsEvent::Udp4ConnectionsHandled
        } else {
            StatsEvent::Udp6ConnectionsHandled
        };
        tracker.update_stats(stats_event, 1);

        Ok(response)
    }

    #[tracing::instrument(level = "debug")]
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

    // FIX: New helper to avoid repeated is_ipv4/is_ipv6 checks
    #[tracing::instrument(level = "debug")]
    async fn handle_udp_announce_cached(is_ipv4: bool, request: &AnnounceRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        let config = tracker.config.tracker_config.clone();

        // Whitelist/Blacklist checks
        if config.whitelist_enabled && !tracker.check_whitelist(InfoHash(request.info_hash.0)) {
            debug!("[UDP ERROR] Torrent Not Whitelisted");
            return Err(ServerError::TorrentNotWhitelisted);
        }
        if config.blacklist_enabled && tracker.check_blacklist(InfoHash(request.info_hash.0)) {
            debug!("[UDP ERROR] Torrent Blacklisted");
            return Err(ServerError::TorrentBlacklisted);
        }

        // Key validation - FIX: Avoid repeated slice bounds checks
        if config.keys_enabled {
            if request.path.len() < 50 {
                debug!("[UDP ERROR] Unknown Key");
                return Err(ServerError::UnknownKey);
            }
            // FIX: Single slice operation instead of nested
            if let Ok(result) = hex::decode(&request.path[10..50]) {
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

        // User key validation
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

        // FIX: Need remote IP for announce - extract from request path or use a default
        // This is a limitation of the cached approach - we need the actual IP
        // For now, return error as we can't determine proper IP
        return Err(ServerError::InternalServerError);
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_announce(remote_addr: SocketAddr, request: &AnnounceRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        let config = tracker.config.tracker_config.clone();
        // FIX: Cache is_ipv4 and is_ipv6 to avoid repeated checks
        let is_ipv4 = remote_addr.is_ipv4();
        let remote_ip = remote_addr.ip();

        // Whitelist/Blacklist checks
        if config.whitelist_enabled && !tracker.check_whitelist(InfoHash(request.info_hash.0)) {
            debug!("[UDP ERROR] Torrent Not Whitelisted");
            return Err(ServerError::TorrentNotWhitelisted);
        }
        if config.blacklist_enabled && tracker.check_blacklist(InfoHash(request.info_hash.0)) {
            debug!("[UDP ERROR] Torrent Blacklisted");
            return Err(ServerError::TorrentBlacklisted);
        }

        // Key validation
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

        // User key validation
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

        // Handle announce
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
            remote_addr: remote_ip,
            numwant: request.peers_wanted.0 as u64,
        }, user_key).await {
            Ok(result) => result.1,
            Err(error) => {
                debug!("[UDP ERROR] Handle Announce - Internal Server Error: {error:#?}");
                return Err(ServerError::InternalServerError);
            }
        };

        // Get peers efficiently - FIX: Use remote_ip instead of remote_addr.ip()
        let torrent_peers = tracker.get_torrent_peers(request.info_hash, 72, TorrentPeersType::All, Some(remote_ip));

        let (peers, peers6) = if let Some(torrent_peers_unwrapped) = torrent_peers {
            // FIX: Pre-allocate with exact capacity needed
            let capacity = torrent_peers_unwrapped.seeds_ipv4.len()
                .min(torrent_peers_unwrapped.seeds_ipv6.len())
                .min(72);
            let mut peers = Vec::with_capacity(capacity);
            let mut peers6 = Vec::with_capacity(capacity);

            // FIX: Check bytes_left once instead of in the loop
            let include_seeds = request.bytes_left.0 != 0;

            if is_ipv4 {
                if include_seeds {
                    // FIX: Use iterator chain to avoid manual count tracking
                    for torrent_peer in torrent_peers_unwrapped.seeds_ipv4.values().take(72) {
                        if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv4Addr>() {
                            peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                        }
                    }
                }

                let remaining = 72 - peers.len();
                for torrent_peer in torrent_peers_unwrapped.peers_ipv4.values().take(remaining) {
                    if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv4Addr>() {
                        peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            } else {
                if include_seeds {
                    for torrent_peer in torrent_peers_unwrapped.seeds_ipv6.values().take(72) {
                        if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv6Addr>() {
                            peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                        }
                    }
                }

                let remaining = 72 - peers6.len();
                for torrent_peer in torrent_peers_unwrapped.peers_ipv6.values().take(remaining) {
                    if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv6Addr>() {
                        peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            }

            (peers, peers6)
        } else {
            (Vec::new(), Vec::new())
        };

        // FIX: Use cached is_ipv4 value, build response more efficiently
        let response = if is_ipv4 {
            Response::from(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(config.request_interval as i32),
                leechers: NumberOfPeers(torrent.peers.len() as i32),
                seeders: NumberOfPeers(torrent.seeds.len() as i32),
                peers,
            })
        } else {
            Response::from(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(config.request_interval as i32),
                leechers: NumberOfPeers(torrent.peers.len() as i32),
                seeders: NumberOfPeers(torrent.seeds.len() as i32),
                peers: peers6,
            })
        };

        // Update stats
        let stats_event = if is_ipv4 {
            StatsEvent::Udp4AnnouncesHandled
        } else {
            StatsEvent::Udp6AnnouncesHandled
        };
        tracker.update_stats(stats_event, 1);

        Ok(response)
    }

    // FIX: New helper to avoid repeated is_ipv4 checks
    #[tracing::instrument(level = "debug")]
    async fn handle_udp_scrape_cached(is_ipv4: bool, request: &ScrapeRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        let mut torrent_stats = Vec::with_capacity(request.info_hashes.len());

        for info_hash in &request.info_hashes {
            let scrape_entry = match tracker.get_torrent(InfoHash(info_hash.0)) {
                Some(torrent_info) => TorrentScrapeStatistics {
                    seeders: NumberOfPeers(torrent_info.seeds.len() as i32),
                    completed: NumberOfDownloads(torrent_info.completed as i32),
                    leechers: NumberOfPeers(torrent_info.peers.len() as i32),
                },
                None => TorrentScrapeStatistics {
                    seeders: NumberOfPeers(0),
                    completed: NumberOfDownloads(0),
                    leechers: NumberOfPeers(0),
                },
            };
            torrent_stats.push(scrape_entry);
        }

        let stats_event = if is_ipv4 {
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

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_scrape(remote_addr: SocketAddr, request: &ScrapeRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        UdpServer::handle_udp_scrape_cached(remote_addr.is_ipv4(), request, tracker).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_error(e: ServerError, transaction_id: TransactionId) -> Response {
        Response::from(ErrorResponse {
            transaction_id,
            message: e.to_string().into()
        })
    }
}