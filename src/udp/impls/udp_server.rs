use std::io::Cursor;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::SystemTime;
use log::{debug, info, warn};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
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
use crate::udp::structs::packet_job::PacketJob;
use crate::udp::structs::port::Port;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::scrape_request::ScrapeRequest;
use crate::udp::structs::scrape_response::ScrapeResponse;
use crate::udp::structs::torrent_scrape_statistics::TorrentScrapeStatistics;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::MAX_SCRAPE_TORRENTS;

impl UdpServer {
    #[tracing::instrument(level = "debug")]
    pub async fn new(
        tracker: Arc<TorrentTracker>,
        bind_address: SocketAddr,
        recv_buffer_size: usize,
        send_buffer_size: usize,
        reuse_address: bool,
        receiver_threads: usize,
        worker_threads: usize,
        queue_size: usize
    ) -> tokio::io::Result<UdpServer>
    {
        let domain = if bind_address.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

        socket.set_recv_buffer_size(recv_buffer_size).map_err(tokio::io::Error::other)?;
        socket.set_send_buffer_size(send_buffer_size).map_err(tokio::io::Error::other)?;
        socket.set_reuse_address(reuse_address).map_err(tokio::io::Error::other)?;
        socket.bind(&bind_address.into()).map_err(tokio::io::Error::other)?;
        socket.set_nonblocking(true).map_err(tokio::io::Error::other)?;

        // Convert to std::net::UdpSocket, then to tokio::net::UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();
        let tokio_socket = UdpSocket::from_std(std_socket)?;

        Ok(UdpServer {
            socket: Arc::new(tokio_socket),
            tracker,
            receiver_threads: receiver_threads as u64,
            worker_threads: worker_threads as u64,
            queue_size: queue_size as u64
        })
    }

    #[tracing::instrument(level = "debug")]
    pub async fn start(&self, rx: tokio::sync::watch::Receiver<bool>) {
        let (packet_tx, packet_rx) = mpsc::channel::<PacketJob>(self.queue_size as usize);
        let packet_rx = Arc::new(tokio::sync::Mutex::new(packet_rx));

        let receiver_threads = self.receiver_threads as usize;
        let worker_threads = self.worker_threads as usize;

        for thread_id in 0..receiver_threads {
            let socket_clone = self.socket.clone();
            let tracker = self.tracker.clone();
            let mut shutdown_rx = rx.clone();
            let packet_tx = packet_tx.clone();

            tokio::spawn(async move {
                info!("Starting UDP receiver thread {}", thread_id);
                let mut data = [0; 1496];

                loop {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {
                            info!("Stopping UDP receiver thread {}...", thread_id);
                            break;
                        }
                        Ok((valid_bytes, remote_addr)) = socket_clone.recv_from(&mut data) => {
                            let payload = data[..valid_bytes].to_vec();

                            debug!("Receiver {} got {} bytes from {}", thread_id, payload.len(), remote_addr);

                            let job = PacketJob {
                                data: payload,
                                remote_addr,
                            };

                            if let Err(e) = packet_tx.try_send(job) {
                                warn!("Packet queue full, dropping packet: {}", e);
                                match remote_addr {
                                    SocketAddr::V4(_) => tracker.update_stats(StatsEvent::Udp4BadRequest, 1),
                                    SocketAddr::V6(_) => tracker.update_stats(StatsEvent::Udp6BadRequest, 1),
                                };
                            }
                        }
                    }
                }
            });
        }

        for thread_id in 0..worker_threads {
            let socket_clone = self.socket.clone();
            let tracker = self.tracker.clone();
            let mut shutdown_rx = rx.clone();
            let packet_rx = packet_rx.clone();

            tokio::spawn(async move {
                info!("Starting UDP worker thread {}", thread_id);

                loop {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {
                            info!("Stopping UDP worker thread {}...", thread_id);
                            break;
                        }
                        job = async {
                            let mut rx = packet_rx.lock().await;
                            rx.recv().await
                        } => {
                            if let Some(PacketJob { data, remote_addr }) = job {
                                debug!("Worker {} processing packet from {}", thread_id, remote_addr);

                                let response = UdpServer::handle_packet(
                                    remote_addr,
                                    data,
                                    tracker.clone()
                                ).await;

                                UdpServer::send_response(
                                    tracker.clone(),
                                    socket_clone.clone(),
                                    remote_addr,
                                    response
                                ).await;
                            }
                        }
                    }
                }
            });
        }

        drop(packet_tx);
    }

    // Optimized send_response with pre-sized buffer
    #[tracing::instrument(level = "debug")]
    pub async fn send_response(
        tracker: Arc<TorrentTracker>,
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        response: Response
    ) {
        debug!("sending response to: {:?}", &remote_addr);
        let sentry = sentry::TransactionContext::new("udp server", "send response");
        let transaction = sentry::start_transaction(sentry);

        // Optimize buffer allocation based on response type
        let estimated_size = match &response {
            Response::Connect(_) => 16,
            Response::AnnounceIpv4(_) => 20 + 6 * 72,  // header + max IPv4 peers (6 bytes each)
            Response::AnnounceIpv6(_) => 20 + 18 * 72, // header + max IPv6 peers (18 bytes each)
            Response::Scrape(_) => 8 + 12 * 74,        // header + max torrents
            Response::Error(_) => 128,                 // reasonable max for error message
        };

        let mut buffer = Vec::with_capacity(estimated_size);
        let mut cursor = Cursor::new(&mut buffer);

        match response.write(&mut cursor) {
            Ok(_) => {
                let position = cursor.position() as usize;
                debug!("Response bytes: {:?}", &buffer[..position]);
                UdpServer::send_packet(socket, &remote_addr, &buffer[..position]).await;
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

    // Rest of the methods remain the same...
    #[tracing::instrument(level = "debug")]
    pub async fn send_packet(socket: Arc<UdpSocket>, remote_addr: &SocketAddr, payload: &[u8]) {
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
        let response = Response::Connect(ConnectResponse {
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
            Response::AnnounceIpv6(AnnounceResponse {
                transaction_id: request.transaction_id,
                announce_interval: AnnounceInterval(config.request_interval as i32),
                leechers: NumberOfPeers(torrent.peers.len() as i32),
                seeders: NumberOfPeers(torrent.seeds.len() as i32),
                peers: peers6,
            })
        } else {
            Response::AnnounceIpv4(AnnounceResponse {
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

        Ok(Response::Scrape(ScrapeResponse {
            transaction_id: request.transaction_id,
            torrent_stats,
        }))
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_udp_error(e: ServerError, transaction_id: TransactionId) -> Response {
        Response::Error(ErrorResponse {
            transaction_id,
            message: e.to_string().into()
        })
    }
}