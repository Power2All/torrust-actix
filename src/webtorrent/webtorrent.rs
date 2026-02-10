use crate::common::structs::custom_error::CustomError;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::config::enums::cluster_mode::ClusterMode;
use crate::security::security::{
    validate_info_hash_hex,
    validate_peer_id_hex,
    validate_webrtc_sdp,
    MAX_OFFER_ID_LENGTH
};
use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::announce_query_request::AnnounceQueryRequest;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::websocket::{
    create_cluster_error_response_json,
    forward_request
};
use crate::webtorrent::structs::webtorrent_server::WebTorrentConnection;
use crate::webtorrent::structs::webtorrent_service_data::WebTorrentServiceData;
use crate::webtorrent::structs::wt_announce::WtAnnounce;
use crate::webtorrent::structs::wt_announce_response::WtAnnounceResponse;
use crate::webtorrent::structs::wt_answer::WtAnswer;
use crate::webtorrent::structs::wt_answer_response::WtAnswerResponse;
use crate::webtorrent::structs::wt_offer::WtOffer;
use crate::webtorrent::structs::wt_offer_response::WtOfferResponse;
use crate::webtorrent::structs::wt_peer_info::WtPeerInfo;
use crate::webtorrent::structs::wt_scrape::WtScrape;
use crate::webtorrent::structs::wt_scrape_info::WtScrapeInfo;
use crate::webtorrent::structs::wt_scrape_response::WtScrapeResponse;
use actix_web::dev::ServerHandle;
use actix_web::{
    web,
    App,
    Error,
    HttpRequest,
    HttpResponse,
    HttpServer
};
use actix_web_actors::ws;
use log::{
    debug,
    error,
    info
};
use std::net::{
    IpAddr,
    SocketAddr
};
use std::sync::Arc;
use std::time::Duration;

pub fn default_u64() -> u64 {
    0
}

pub fn default_u16() -> u16 {
    0
}

const WT_MAX_PEERS: u64 = 50;

pub async fn handle_webtorrent_announce(
    tracker: &TorrentTracker,
    announce: WtAnnounce,
    ip: IpAddr,
) -> Result<WtAnnounceResponse, CustomError> {
    debug!("[WEBTORRENT] Handling announce for info_hash: {} (length: {})", announce.info_hash, announce.info_hash.len());
    validate_info_hash_hex(&announce.info_hash)?;
    validate_peer_id_hex(&announce.peer_id)?;
    if let Some(ref offer) = announce.offer {
        validate_webrtc_sdp(offer)?;
    }
    if let Some(ref offer_id) = announce.offer_id && offer_id.len() > MAX_OFFER_ID_LENGTH {
        return Err(CustomError::new("offer_id exceeds maximum length"));
    }
    if tracker.config.tracker_config.cluster == ClusterMode::slave {
        return handle_webtorrent_announce_cluster_forward(tracker, announce, ip).await;
    }
    let info_hash_bytes = if announce.info_hash.len() == 40 {
        hex::decode(&announce.info_hash)
            .map_err(|_| CustomError::new("Invalid info_hash: not hex"))?
    } else {
        let bytes: Vec<u8> = announce.info_hash.bytes().collect();
        if bytes.len() >= 20 {
            bytes[..20].to_vec()
        } else {
            return Err(CustomError::new(&format!("Invalid info_hash: too short ({} bytes)", bytes.len())));
        }
    };
    if info_hash_bytes.len() != 20 {
        return Err(CustomError::new("Invalid info_hash: wrong length"));
    }
    let mut info_hash_array = [0u8; 20];
    info_hash_array.copy_from_slice(&info_hash_bytes);
    let info_hash = InfoHash(info_hash_array);
    let peer_id_bytes = if announce.peer_id.len() == 40 {
        hex::decode(&announce.peer_id)
            .map_err(|_| CustomError::new("Invalid peer_id: not hex"))?
    } else {
        let bytes: Vec<u8> = announce.peer_id.bytes().collect();
        if bytes.len() >= 20 {
            bytes[..20].to_vec()
        } else {
            return Err(CustomError::new(&format!("Invalid peer_id: too short ({} bytes)", bytes.len())));
        }
    };
    if peer_id_bytes.len() != 20 {
        return Err(CustomError::new("Invalid peer_id: wrong length"));
    }
    let mut peer_id_array = [0u8; 20];
    peer_id_array.copy_from_slice(&peer_id_bytes);
    let peer_id = PeerId(peer_id_array);
    if tracker.config.tracker_config.whitelist_enabled
        && !tracker.check_whitelist(info_hash) {
        return Ok(WtAnnounceResponse {
            info_hash: announce.info_hash.clone(),
            complete: 0,
            incomplete: 0,
            peers: vec![],
            interval: tracker.config.tracker_config.request_interval as i64,
            failure_reason: Some("Torrent not in whitelist".to_string()),
            warning_message: None,
        });
    }
    if tracker.config.tracker_config.blacklist_enabled
        && tracker.check_blacklist(info_hash) {
        return Ok(WtAnnounceResponse {
            info_hash: announce.info_hash.clone(),
            complete: 0,
            incomplete: 0,
            peers: vec![],
            interval: tracker.config.tracker_config.request_interval as i64,
            failure_reason: Some("Torrent is blacklisted".to_string()),
            warning_message: None,
        });
    }
    let event = match announce.event.as_deref() {
        Some("start") => AnnounceEvent::Started,
        Some("stop") => AnnounceEvent::Stopped,
        Some("complete") => AnnounceEvent::Completed,
        None => AnnounceEvent::None,
        _ => AnnounceEvent::Started,
    };
    let port = if announce.port > 0 { announce.port } else { 6881 };
    let announce_query = AnnounceQueryRequest {
        info_hash,
        peer_id,
        port,
        uploaded: announce.uploaded,
        downloaded: announce.downloaded,
        left: announce.left.unwrap_or(0),
        compact: false,
        no_peer_id: false,
        event,
        remote_addr: ip,
        numwant: announce.numwant.unwrap_or(WT_MAX_PEERS as i64) as u64,
    };
    let tracker_ref = unsafe {
        Arc::from_raw(tracker as *const TorrentTracker)
    };
    let (_torrent_peer, torrent_entry) = tracker.handle_announce(
        Arc::clone(&tracker_ref),
        announce_query,
        None
    ).await?;
    if announce.offer.is_some() || announce.offer_id.is_some() {
        let webrtc_offer = announce.offer.clone();
        let webrtc_offer_id = announce.offer_id.clone();
        let updated_peer = TorrentPeer {
            peer_id,
            peer_addr: SocketAddr::new(ip, port),
            updated: std::time::Instant::now(),
            uploaded: NumberOfBytes(announce.uploaded as i64),
            downloaded: NumberOfBytes(announce.downloaded as i64),
            left: NumberOfBytes(announce.left.unwrap_or(0) as i64),
            event: AnnounceEvent::None,
            webrtc_offer,
            webrtc_offer_id,
            is_webtorrent: true,
        };
        tracker.add_torrent_peer(info_hash, peer_id, updated_peer, false);
    }
    std::mem::forget(tracker_ref);
    let peers_type = if ip.is_ipv4() {
        TorrentPeersType::IPv4
    } else {
        TorrentPeersType::IPv6
    };
    let numwant = announce.numwant.unwrap_or(WT_MAX_PEERS as i64) as usize;
    let numwant = numwant.min(WT_MAX_PEERS as usize);
    let peers = tracker.get_peers(&torrent_entry.peers, peers_type, Some(ip), numwant);
    let offers_only = announce.offers_only.unwrap_or(false);
    let wt_peers: Vec<WtPeerInfo> = peers
        .iter()
        .filter(|(_, peer)| !offers_only || peer.webrtc_offer.is_some())
        .map(|(peer_id, peer)| {
            WtPeerInfo {
                peer_id: hex::encode(peer_id.0),
                offer: peer.webrtc_offer.clone(),
                offer_id: peer.webrtc_offer_id.clone(),
                ip: Some(peer.peer_addr.ip().to_string()),
                port: Some(peer.peer_addr.port()),
            }
        })
        .collect();
    Ok(WtAnnounceResponse {
        info_hash: announce.info_hash,
        complete: torrent_entry.seeds.len() as i64,
        incomplete: torrent_entry.peers.len() as i64,
        peers: wt_peers,
        interval: tracker.config.tracker_config.request_interval as i64,
        failure_reason: None,
        warning_message: None,
    })
}

async fn handle_webtorrent_announce_cluster_forward(
    tracker: &TorrentTracker,
    announce: WtAnnounce,
    ip: IpAddr,
) -> Result<WtAnnounceResponse, CustomError> {
    let protocol = ProtocolType::WebTorrentHttp;
    let client_port = announce.port;
    let payload = serde_json::to_vec(&announce)
        .map_err(|e| CustomError::new(&format!("Failed to serialize announce: {}", e)))?;
    let tracker_ref = unsafe {
        Arc::from_raw(tracker as *const TorrentTracker)
    };
    let result = forward_request(
        &tracker_ref,
        protocol,
        RequestType::WtAnnounce,
        ip,
        client_port,
        payload,
    ).await;
    std::mem::forget(tracker_ref);
    match result {
        Ok(response) => {
            serde_json::from_slice(&response.payload)
                .map_err(|e| CustomError::new(&format!("Failed to parse master response: {}", e)))
        }
        Err(e) => {
            Ok(WtAnnounceResponse {
                info_hash: announce.info_hash.clone(),
                complete: 0,
                incomplete: 0,
                peers: vec![],
                interval: tracker.config.tracker_config.request_interval as i64,
                failure_reason: Some(create_cluster_error_response_json(&e)),
                warning_message: None,
            })
        }
    }
}

pub async fn handle_webtorrent_scrape(
    tracker: &TorrentTracker,
    scrape: WtScrape,
) -> Result<WtScrapeResponse, CustomError> {
    debug!("[WEBTORRENT] Handling scrape for {} torrents", scrape.info_hash.len());
    if tracker.config.tracker_config.cluster == ClusterMode::slave {
        return handle_webtorrent_scrape_cluster_forward(tracker, scrape).await;
    }
    let mut files = std::collections::HashMap::new();
    for info_hash_hex in &scrape.info_hash {
        let info_hash_bytes = match hex::decode(info_hash_hex) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if info_hash_bytes.len() != 20 {
            continue;
        }
        let mut info_hash_array = [0u8; 20];
        info_hash_array.copy_from_slice(&info_hash_bytes);
        let info_hash = InfoHash(info_hash_array);
        if tracker.config.tracker_config.whitelist_enabled
            && !tracker.check_whitelist(info_hash) {
            continue;
        }
        if tracker.config.tracker_config.blacklist_enabled
            && tracker.check_blacklist(info_hash) {
            continue;
        }
        let torrent_entry = tracker.get_torrent(info_hash).unwrap_or_default();
        files.insert(info_hash_hex.clone(), WtScrapeInfo {
            complete: torrent_entry.seeds.len() as i64,
            downloaded: torrent_entry.completed as i64,
            incomplete: torrent_entry.peers.len() as i64,
        });
    }
    Ok(WtScrapeResponse { files })
}

async fn handle_webtorrent_scrape_cluster_forward(
    tracker: &TorrentTracker,
    scrape: WtScrape,
) -> Result<WtScrapeResponse, CustomError> {
    let protocol = ProtocolType::WebTorrentHttp;
    let client_port = 0;
    let payload = serde_json::to_vec(&scrape)
        .map_err(|e| CustomError::new(&format!("Failed to serialize scrape: {}", e)))?;
    let tracker_ref = unsafe {
        Arc::from_raw(tracker as *const TorrentTracker)
    };
    let result = forward_request(
        &tracker_ref,
        protocol,
        RequestType::WtScrape,
        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        client_port,
        payload,
    ).await;
    std::mem::forget(tracker_ref);
    match result {
        Ok(response) => {
            serde_json::from_slice(&response.payload)
                .map_err(|e| CustomError::new(&format!("Failed to parse master response: {}", e)))
        }
        Err(_e) => {
            Ok(WtScrapeResponse { files: std::collections::HashMap::new() })
        }
    }
}

pub async fn handle_webtorrent_offer(
    tracker: &TorrentTracker,
    offer: WtOffer,
    ip: IpAddr,
) -> Result<WtOfferResponse, CustomError> {
    info!("[WEBTORRENT] Handling offer for info_hash: {}, peer_id: {}, offer_id: {}", offer.info_hash, offer.peer_id, offer.offer_id);
    validate_info_hash_hex(&offer.info_hash)?;
    validate_peer_id_hex(&offer.peer_id)?;
    validate_webrtc_sdp(&offer.offer)?;
    if offer.offer_id.len() > MAX_OFFER_ID_LENGTH {
        return Err(CustomError::new("offer_id exceeds maximum length"));
    }
    if tracker.config.tracker_config.cluster == ClusterMode::slave {
        return handle_webtorrent_offer_cluster_forward(tracker, offer, ip).await;
    }
    let info_hash_bytes = if offer.info_hash.len() == 40 {
        hex::decode(&offer.info_hash)
            .map_err(|_| CustomError::new("Invalid info_hash: not hex"))?
    } else {
        let bytes: Vec<u8> = offer.info_hash.bytes().collect();
        if bytes.len() >= 20 {
            bytes[..20].to_vec()
        } else {
            return Err(CustomError::new(&format!("Invalid info_hash: too short ({} bytes)", bytes.len())));
        }
    };
    if info_hash_bytes.len() != 20 {
        return Err(CustomError::new("Invalid info_hash: wrong length"));
    }
    let mut info_hash_array = [0u8; 20];
    info_hash_array.copy_from_slice(&info_hash_bytes);
    let info_hash = InfoHash(info_hash_array);
    let peer_id_bytes = if offer.peer_id.len() == 40 {
        hex::decode(&offer.peer_id)
            .map_err(|_| CustomError::new("Invalid peer_id: not hex"))?
    } else {
        let bytes: Vec<u8> = offer.peer_id.bytes().collect();
        if bytes.len() >= 20 {
            bytes[..20].to_vec()
        } else {
            return Err(CustomError::new(&format!("Invalid peer_id: too short ({} bytes)", bytes.len())));
        }
    };
    if peer_id_bytes.len() != 20 {
        return Err(CustomError::new("Invalid peer_id: wrong length"));
    }
    let mut peer_id_array = [0u8; 20];
    peer_id_array.copy_from_slice(&peer_id_bytes);
    let peer_id = PeerId(peer_id_array);
    if tracker.config.tracker_config.whitelist_enabled
        && !tracker.check_whitelist(info_hash) {
        return Ok(WtOfferResponse {
            info_hash: offer.info_hash.clone(),
            peer_id: offer.peer_id.clone(),
            offer_id: offer.offer_id.clone(),
            error: Some("Torrent not in whitelist".to_string()),
        });
    }
    if tracker.config.tracker_config.blacklist_enabled
        && tracker.check_blacklist(info_hash) {
        return Ok(WtOfferResponse {
            info_hash: offer.info_hash.clone(),
            peer_id: offer.peer_id.clone(),
            offer_id: offer.offer_id.clone(),
            error: Some("Torrent is blacklisted".to_string()),
        });
    }
    let updated_peer = TorrentPeer {
        peer_id,
        peer_addr: SocketAddr::new(ip, 0),
        updated: std::time::Instant::now(),
        uploaded: NumberOfBytes(0),
        downloaded: NumberOfBytes(0),
        left: NumberOfBytes(0),
        event: AnnounceEvent::None,
        webrtc_offer: Some(offer.offer.clone()),
        webrtc_offer_id: Some(offer.offer_id.clone()),
        is_webtorrent: true,
    };
    tracker.add_torrent_peer(info_hash, peer_id, updated_peer, false);
    info!(
        "[WEBTORRENT] Stored WebRTC offer {} for peer {} on torrent {}",
        offer.offer_id, offer.peer_id, offer.info_hash
    );
    Ok(WtOfferResponse {
        info_hash: offer.info_hash,
        peer_id: offer.peer_id,
        offer_id: offer.offer_id,
        error: None,
    })
}

async fn handle_webtorrent_offer_cluster_forward(
    tracker: &TorrentTracker,
    offer: WtOffer,
    ip: IpAddr,
) -> Result<WtOfferResponse, CustomError> {
    let protocol = ProtocolType::WebTorrentHttp;
    let client_port = 0;
    let payload = serde_json::to_vec(&offer)
        .map_err(|e| CustomError::new(&format!("Failed to serialize offer: {}", e)))?;
    let tracker_ref = unsafe {
        Arc::from_raw(tracker as *const TorrentTracker)
    };
    let result = forward_request(
        &tracker_ref,
        protocol,
        RequestType::WtOffer,
        ip,
        client_port,
        payload,
    ).await;
    std::mem::forget(tracker_ref);
    match result {
        Ok(response) => {
            serde_json::from_slice(&response.payload)
                .map_err(|e| CustomError::new(&format!("Failed to parse master response: {}", e)))
        }
        Err(e) => {
            Ok(WtOfferResponse {
                info_hash: offer.info_hash.clone(),
                peer_id: offer.peer_id.clone(),
                offer_id: offer.offer_id.clone(),
                error: Some(create_cluster_error_response_json(&e)),
            })
        }
    }
}

pub async fn handle_webtorrent_answer(
    tracker: &TorrentTracker,
    answer: WtAnswer,
    ip: IpAddr,
) -> Result<WtAnswerResponse, CustomError> {
    info!(
        "[WEBTORRENT] Handling answer for info_hash: {}, from_peer_id: {}, to_peer_id: {}, offer_id: {}",
        answer.info_hash, answer.peer_id, answer.to_peer_id, answer.offer_id
    );
    if tracker.config.tracker_config.cluster == ClusterMode::slave {
        return handle_webtorrent_answer_cluster_forward(tracker, answer, ip).await;
    }
    let info_hash_bytes = if answer.info_hash.len() == 40 {
        hex::decode(&answer.info_hash)
            .map_err(|_| CustomError::new("Invalid info_hash: not hex"))?
    } else {
        let bytes: Vec<u8> = answer.info_hash.bytes().collect();
        if bytes.len() >= 20 {
            bytes[..20].to_vec()
        } else {
            return Err(CustomError::new(&format!("Invalid info_hash: too short ({} bytes)", bytes.len())));
        }
    };
    if info_hash_bytes.len() != 20 {
        return Err(CustomError::new("Invalid info_hash: wrong length"));
    }
    let mut info_hash_array = [0u8; 20];
    info_hash_array.copy_from_slice(&info_hash_bytes);
    let info_hash = InfoHash(info_hash_array);
    let to_peer_id_bytes = if answer.to_peer_id.len() == 40 {
        hex::decode(&answer.to_peer_id)
            .map_err(|_| CustomError::new("Invalid to_peer_id: not hex"))?
    } else {
        let bytes: Vec<u8> = answer.to_peer_id.bytes().collect();
        if bytes.len() >= 20 {
            bytes[..20].to_vec()
        } else {
            return Err(CustomError::new(&format!("Invalid to_peer_id: too short ({} bytes)", bytes.len())));
        }
    };
    if to_peer_id_bytes.len() != 20 {
        return Err(CustomError::new("Invalid to_peer_id: wrong length"));
    }
    let mut to_peer_id_array = [0u8; 20];
    to_peer_id_array.copy_from_slice(&to_peer_id_bytes);
    let to_peer_id = PeerId(to_peer_id_array);
    if tracker.config.tracker_config.whitelist_enabled
        && !tracker.check_whitelist(info_hash) {
        return Ok(WtAnswerResponse {
            info_hash: answer.info_hash.clone(),
            peer_id: answer.peer_id.clone(),
            to_peer_id: answer.to_peer_id.clone(),
            offer_id: answer.offer_id.clone(),
            error: Some("Torrent not in whitelist".to_string()),
        });
    }
    if tracker.config.tracker_config.blacklist_enabled
        && tracker.check_blacklist(info_hash) {
        return Ok(WtAnswerResponse {
            info_hash: answer.info_hash.clone(),
            peer_id: answer.peer_id.clone(),
            to_peer_id: answer.to_peer_id.clone(),
            offer_id: answer.offer_id.clone(),
            error: Some("Torrent is blacklisted".to_string()),
        });
    }
    if let Some(torrent_entry) = tracker.get_torrent(info_hash) {
        let peer_exists = torrent_entry.peers.contains_key(&to_peer_id) ||
                         torrent_entry.seeds.contains_key(&to_peer_id);
        if peer_exists {
            info!(
                "[WEBTORRENT] Answer for offer_id {} destined for peer {} (peer exists in swarm)",
                answer.offer_id, answer.to_peer_id
            );
        } else {
            info!(
                "[WEBTORRENT] Answer for offer_id {} destined for peer {} but peer not found in swarm",
                answer.offer_id, answer.to_peer_id
            );
        }
    } else {
        info!(
            "[WEBTORRENT] Answer for offer_id {} but torrent {} not found",
            answer.offer_id, answer.info_hash
        );
    }
    Ok(WtAnswerResponse {
        info_hash: answer.info_hash,
        peer_id: answer.peer_id,
        to_peer_id: answer.to_peer_id,
        offer_id: answer.offer_id,
        error: None,
    })
}

async fn handle_webtorrent_answer_cluster_forward(
    tracker: &TorrentTracker,
    answer: WtAnswer,
    ip: IpAddr,
) -> Result<WtAnswerResponse, CustomError> {
    let protocol = ProtocolType::WebTorrentHttp;
    let client_port = 0;
    let payload = serde_json::to_vec(&answer)
        .map_err(|e| CustomError::new(&format!("Failed to serialize answer: {}", e)))?;
    let tracker_ref = unsafe {
        Arc::from_raw(tracker as *const TorrentTracker)
    };
    let result = forward_request(
        &tracker_ref,
        protocol,
        RequestType::WtAnswer,
        ip,
        client_port,
        payload,
    ).await;
    std::mem::forget(tracker_ref);
    match result {
        Ok(response) => {
            serde_json::from_slice(&response.payload)
                .map_err(|e| CustomError::new(&format!("Failed to parse master response: {}", e)))
        }
        Err(e) => {
            Ok(WtAnswerResponse {
                info_hash: answer.info_hash.clone(),
                peer_id: answer.peer_id.clone(),
                to_peer_id: answer.to_peer_id.clone(),
                offer_id: answer.offer_id.clone(),
                error: Some(create_cluster_error_response_json(&e)),
            })
        }
    }
}

pub async fn webtorrent_websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<Arc<WebTorrentServiceData>>,
) -> Result<HttpResponse, Error> {
    let client_ip = req.peer_addr().map(|addr| addr.ip());
    let connection = WebTorrentConnection::new(data.get_ref().clone(), client_ip);
    ws::start(connection, &req, stream)
}

pub async fn webtorrent_service(
    addr: SocketAddr,
    tracker: Arc<TorrentTracker>,
    config: crate::config::structs::webtorrent_trackers_config::WebTorrentTrackersConfig,
) -> (ServerHandle, impl Future<Output = Result<(), std::io::Error>>) {
    let keep_alive = config.keep_alive;
    let request_timeout = config.request_timeout;
    let disconnect_timeout = config.disconnect_timeout;
    let worker_threads = config.threads as usize;
    let service_data = Arc::new(WebTorrentServiceData {
        torrent_tracker: tracker.clone(),
        webtorrent_config: Arc::new(config.clone()),
    });
    if config.ssl {
        info!("[WEBTORRENT] Starting WSS server on {}", addr);
        if config.ssl_key.is_empty() || config.ssl_cert.is_empty() {
            error!("[WEBTORRENT] No SSL key or SSL certificate given, exiting...");
            panic!("[WEBTORRENT] SSL configuration required for WSS");
        }
        let server_id = ServerIdentifier::WebTorrentTracker(addr.to_string());
        if let Err(e) = tracker.certificate_store.load_certificate(
            server_id.clone(),
            &config.ssl_cert,
            &config.ssl_key,
        ) {
            panic!("[WEBTORRENT] Failed to load SSL certificate: {}", e);
        }
        let resolver = match DynamicCertificateResolver::new(
            Arc::clone(&tracker.certificate_store),
            server_id,
        ) {
            Ok(resolver) => Arc::new(resolver),
            Err(e) => panic!("[WEBTORRENT] Failed to create certificate resolver: {}", e),
        };
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(service_data.clone()))
                .route("/", web::get().to(webtorrent_websocket_handler))
        })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(request_timeout))
        .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
        .workers(worker_threads)
        .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
        .unwrap()
        .disable_signals()
        .run();
        (server.handle(), server)
    } else {
        info!("[WEBTORRENT] Starting WS server on {}", addr);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(service_data.clone()))
                .route("/", web::get().to(webtorrent_websocket_handler))
        })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(request_timeout))
        .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
        .workers(worker_threads)
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();
        (server.handle(), server)
    }
}