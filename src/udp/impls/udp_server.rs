use std::io::Cursor;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::SystemTime;
use log::{debug, info};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::UdpSocket;

#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;

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
use crate::udp::structs::port::Port;
use crate::udp::structs::response_batch_manager::ResponseBatchManager;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::scrape_request::ScrapeRequest;
use crate::udp::structs::scrape_response::ScrapeResponse;
use crate::udp::structs::torrent_scrape_statistics::TorrentScrapeStatistics;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::MAX_SCRAPE_TORRENTS;

impl UdpServer {
    #[tracing::instrument(level = "debug")]
    pub async fn new(tracker: Arc<TorrentTracker>, bind_address: SocketAddr, threads: u64, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool) -> tokio::io::Result<UdpServer>
    {
        let domain = if bind_address.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

        // Aggressive buffer sizing for high throughput
        let actual_recv_buffer = recv_buffer_size.max(16_777_216); // Minimum 16MB
        let actual_send_buffer = send_buffer_size.max(16_777_216); // Minimum 16MB

        socket.set_recv_buffer_size(actual_recv_buffer).map_err(tokio::io::Error::other)?;
        socket.set_send_buffer_size(actual_send_buffer).map_err(tokio::io::Error::other)?;
        socket.set_reuse_address(reuse_address).map_err(tokio::io::Error::other)?;

        // Enable SO_REUSEPORT for better load distribution across threads
        #[cfg(target_os = "linux")]
        {
            let reuse_port = 1i32;
            unsafe {
                let optval = &reuse_port as *const i32 as *const libc::c_void;
                if libc::setsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    optval,
                    std::mem::size_of::<i32>() as libc::socklen_t,
                ) != 0 {
                    log::warn!("Failed to set SO_REUSEPORT - continuing without it");
                }
            }
        }

        socket.bind(&bind_address.into()).map_err(tokio::io::Error::other)?;
        socket.set_nonblocking(true).map_err(tokio::io::Error::other)?;

        // Convert to std::net::UdpSocket, then to tokio::net::UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();
        let tokio_socket = UdpSocket::from_std(std_socket)?;

        // Log actual buffer sizes
        let sock_ref = socket2::SockRef::from(&tokio_socket);
        let actual_recv = sock_ref.recv_buffer_size().unwrap_or(0);
        let actual_send = sock_ref.send_buffer_size().unwrap_or(0);
        info!("Socket created with buffers - Recv: {} bytes, Send: {} bytes", actual_recv, actual_send);

        Ok(UdpServer {
            socket: Arc::new(tokio_socket),
            threads,
            tracker,
        })
    }

    #[tracing::instrument(level = "debug")]
    pub async fn start(&self, rx: tokio::sync::watch::Receiver<bool>)
    {
        let threads = self.threads;
        // Create multiple sockets for better performance using SO_REUSEPORT
        for thread_id in 0..threads {
            let socket_clone = self.socket.clone();
            let tracker = self.tracker.clone();
            let mut rx = rx.clone();

            tokio::spawn(async move {
                // Larger buffer to handle burst traffic
                let mut data = [0; 2048]; // Increased from 1496
                let mut packet_count = 0u64;
                let mut last_stats = std::time::Instant::now();

                loop {
                    let udp_sock = socket_clone.local_addr().unwrap();
                    tokio::select! {
                        _ = rx.changed() => {
                            info!("Stopping UDP server thread {}: {udp_sock}...", thread_id);
                            break;
                        }
                        result = socket_clone.recv_from(&mut data) => {
                            match result {
                                Ok((valid_bytes, remote_addr)) => {
                                    packet_count += 1;

                                    // Log stats every 10k packets per thread
                                    if packet_count % 10000 == 0 {
                                        let elapsed = last_stats.elapsed();
                                        let rate = 10000.0 / elapsed.as_secs_f64();
                                        debug!("Thread {} processed 10k packets in {:?} ({:.1} pps)",
                                              thread_id, elapsed, rate);
                                        last_stats = std::time::Instant::now();
                                    }

                                    let payload = &data[..valid_bytes];
                                    debug!("Thread {} received {} bytes from {}", thread_id, payload.len(), remote_addr);

                                    let tracker_cloned = tracker.clone();
                                    let socket_cloned = socket_clone.clone();
                                    let payload_vec = payload.to_vec();

                                    // Process immediately without extra spawning for better performance
                                    let response = UdpServer::handle_packet(remote_addr, payload_vec, tracker_cloned.clone()).await;
                                    UdpServer::send_response(tracker_cloned.clone(), socket_cloned.clone(), remote_addr, response).await;
                                }
                                Err(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::WouldBlock => {
                                            // This is normal for non-blocking sockets
                                            tokio::task::yield_now().await;
                                        }
                                        _ => {
                                            log::error!("Thread {} recv_from error: {}", thread_id, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn send_response(tracker: Arc<TorrentTracker>, socket: Arc<UdpSocket>, remote_addr: SocketAddr, response: Response)
    {
        debug!("sending response to: {:?}", &remote_addr);
        let sentry = sentry::TransactionContext::new("udp server", "send response");
        let transaction = sentry::start_transaction(sentry);

        // Pre-allocate buffer with exact capacity instead of MAX_PACKET_SIZE
        let mut buffer = Vec::with_capacity(512); // Most responses are much smaller than MAX_PACKET_SIZE
        let mut cursor = Cursor::new(&mut buffer);

        match response.write(&mut cursor) {
            Ok(_) => {
                let position = cursor.position() as usize;
                debug!("Response bytes: {:?}", &buffer[..position]);

                // Get batch manager for this socket and queue the response
                let batch_manager = ResponseBatchManager::get_for_socket(socket).await;
                batch_manager.queue_response(remote_addr, buffer[..position].to_vec());
            }
            Err(error) => {
                sentry::capture_error(&error);
                match remote_addr {
                    SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4InvalidRequest, 1); }
                    SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6InvalidRequest, 1); }
                }
                debug!("could not write response to bytes.");
            }
        }

        transaction.finish();
    }

    #[tracing::instrument(level = "debug")]
    pub async fn send_packet(socket: Arc<UdpSocket>, remote_addr: &SocketAddr, payload: &[u8]) {
        // This method is kept for compatibility but shouldn't be used in the batched version
        let _ = socket.send_to(payload, remote_addr).await;
    }

    #[tracing::instrument(level = "debug")]
    pub async fn get_connection_id(remote_address: &SocketAddr) -> ConnectionId {
        match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => ConnectionId(((duration.as_secs() / 3600) | ((remote_address.port() as u64) << 36)) as i64),
            Err(_) => ConnectionId(0x7FFFFFFFFFFFFFFF)
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_packet(remote_addr: SocketAddr, payload: Vec<u8>, tracker: Arc<TorrentTracker>) -> Response {
        let transaction_id = match Request::from_bytes(&payload, MAX_SCRAPE_TORRENTS) {
            Ok(request) => {
                let tid = match &request {
                    Request::Connect(connect_request) => connect_request.transaction_id,
                    Request::Announce(announce_request) => announce_request.transaction_id,
                    Request::Scrape(scrape_request) => scrape_request.transaction_id,
                };

                match UdpServer::handle_request(request, remote_addr, tracker.clone()).await {
                    Ok(response) => return response,
                    Err(_e) => {
                        match remote_addr {
                            SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4InvalidRequest, 1); }
                            SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6InvalidRequest, 1); }
                        }
                        tid
                    }
                }
            }
            Err(_) => {
                match remote_addr {
                    SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4BadRequest, 1); }
                    SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6BadRequest, 1); }
                }
                TransactionId(0)
            }
        };

        UdpServer::handle_udp_error(ServerError::BadRequest, transaction_id).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_request(request: Request, remote_addr: SocketAddr, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
        let sentry = sentry::TransactionContext::new("udp server", "handle packet");
        let transaction = sentry::start_transaction(sentry);

        let result = match request {
            Request::Connect(connect_request) => {
                UdpServer::handle_udp_connect(remote_addr, &connect_request, tracker).await
            }
            Request::Announce(announce_request) => {
                UdpServer::handle_udp_announce(remote_addr, &announce_request, tracker).await
            }
            Request::Scrape(scrape_request) => {
                UdpServer::handle_udp_scrape(remote_addr, &scrape_request, tracker).await
            }
        };

        transaction.finish();
        result
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

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_announce(remote_addr: SocketAddr, request: &AnnounceRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
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
            remote_addr: remote_addr.ip(),
            numwant: request.peers_wanted.0 as u64,
        }, user_key).await {
            Ok(result) => result.1,
            Err(error) => {
                debug!("[UDP ERROR] Handle Announce - Internal Server Error: {error:#?}");
                return Err(ServerError::InternalServerError);
            }
        };

        // Get peers efficiently
        let torrent_peers = tracker.get_torrent_peers(request.info_hash, 72, TorrentPeersType::All, Some(remote_addr.ip()));

        let (peers, peers6) = if let Some(torrent_peers_unwrapped) = torrent_peers {
            let mut peers = Vec::with_capacity(72);
            let mut peers6 = Vec::with_capacity(72);
            let mut count = 0;

            // Only collect peers if not completed download
            if request.bytes_left.0 != 0 {
                if remote_addr.is_ipv4() {
                    for torrent_peer in torrent_peers_unwrapped.seeds_ipv4.values().take(72) {
                        if count >= 72 { break; }
                        if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv4Addr>() {
                            peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                            count += 1;
                        }
                    }
                } else {
                    for torrent_peer in torrent_peers_unwrapped.seeds_ipv6.values().take(72) {
                        if count >= 72 { break; }
                        if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv6Addr>() {
                            peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                            count += 1;
                        }
                    }
                }
            }

            // Collect regular peers
            if remote_addr.is_ipv4() {
                for torrent_peer in torrent_peers_unwrapped.peers_ipv4.values().take(72 - count) {
                    if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv4Addr>() {
                        peers.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            } else {
                for torrent_peer in torrent_peers_unwrapped.peers_ipv6.values().take(72 - count) {
                    if let Ok(ip) = torrent_peer.peer_addr.ip().to_string().parse::<Ipv6Addr>() {
                        peers6.push(ResponsePeer { ip_address: ip, port: Port(torrent_peer.peer_addr.port()) });
                    }
                }
            }

            (peers, peers6)
        } else {
            (Vec::new(), Vec::new())
        };

        // Create response
        let response = if remote_addr.is_ipv6() {
            Response::from(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(config.request_interval as i32),
                leechers: NumberOfPeers(torrent.peers.len() as i32),
                seeders: NumberOfPeers(torrent.seeds.len() as i32),
                peers: peers6,
            })
        } else {
            Response::from(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(config.request_interval as i32),
                leechers: NumberOfPeers(torrent.peers.len() as i32),
                seeders: NumberOfPeers(torrent.seeds.len() as i32),
                peers,
            })
        };

        // Update stats
        let stats_event = if remote_addr.is_ipv4() {
            StatsEvent::Udp4AnnouncesHandled
        } else {
            StatsEvent::Udp6AnnouncesHandled
        };
        tracker.update_stats(stats_event, 1);

        Ok(response)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_scrape(remote_addr: SocketAddr, request: &ScrapeRequest, tracker: Arc<TorrentTracker>) -> Result<Response, ServerError> {
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

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_error(e: ServerError, transaction_id: TransactionId) -> Response {
        Response::from(ErrorResponse {
            transaction_id,
            message: e.to_string().into()
        })
    }
}