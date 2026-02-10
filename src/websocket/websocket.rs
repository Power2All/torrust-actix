use crate::common::common::parse_query;
use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::udp_server::UdpServer;
use crate::websocket::enums::encoding_error::EncodingError;
use crate::websocket::enums::forward_error::ForwardError;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::structs::announce_response_stats::AnnounceResponseStats;
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::cluster_response::ClusterResponse;
use crate::websocket::structs::slave_client_state::SlaveClientState;
use crate::websocket::structs::websocket_service_data::WebSocketServiceData;
use crate::websocket::types::SlaveSenderChannel;
use crate::webtorrent::structs::wt_announce::WtAnnounce;
use crate::webtorrent::structs::wt_announce_response::WtAnnounceResponse;
use crate::webtorrent::structs::wt_offer::WtOffer;
use crate::webtorrent::structs::wt_offer_response::WtOfferResponse;
use crate::webtorrent::structs::wt_answer::WtAnswer;
use crate::webtorrent::structs::wt_answer_response::WtAnswerResponse;
use crate::webtorrent::structs::wt_scrape::WtScrape;
use crate::webtorrent::structs::wt_scrape_response::WtScrapeResponse;
use crate::webtorrent::webtorrent::{
    handle_webtorrent_announce,
    handle_webtorrent_scrape,
    handle_webtorrent_offer,
    handle_webtorrent_answer
};
use bip_bencode::{
    ben_bytes,
    ben_int,
    ben_list,
    ben_map,
    BMutAccess
};
use log::{
    debug,
    error,
    info,
    warn
};
use serde::{
    de::DeserializeOwned,
    Serialize
};
use std::borrow::Cow;
use std::io::Write;
use std::net::{
    IpAddr,
    SocketAddr
};
use std::sync::Arc;

pub fn encode<T: Serialize>(encoding: &ClusterEncoding, value: &T) -> Result<Vec<u8>, EncodingError> {
    match encoding {
        ClusterEncoding::binary => encode_binary(value),
        ClusterEncoding::json => encode_json(value),
        ClusterEncoding::msgpack => encode_msgpack(value),
    }
}

pub fn decode<T: DeserializeOwned>(encoding: &ClusterEncoding, data: &[u8]) -> Result<T, EncodingError> {
    match encoding {
        ClusterEncoding::binary => decode_binary(data),
        ClusterEncoding::json => decode_json(data),
        ClusterEncoding::msgpack => decode_msgpack(data),
    }
}

fn encode_binary<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    rmp_serde::to_vec(value)
        .map_err(|e| EncodingError::SerializationError(e.to_string()))
}

fn decode_binary<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    rmp_serde::from_slice(data)
        .map_err(|e| EncodingError::DeserializationError(e.to_string()))
}

fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    serde_json::to_vec(value)
        .map_err(|e| EncodingError::SerializationError(e.to_string()))
}

fn decode_json<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    serde_json::from_slice(data)
        .map_err(|e| EncodingError::DeserializationError(e.to_string()))
}

fn encode_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    rmp_serde::to_vec(value)
        .map_err(|e| EncodingError::SerializationError(e.to_string()))
}

fn decode_msgpack<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    rmp_serde::from_slice(data)
        .map_err(|e| EncodingError::DeserializationError(e.to_string()))
}

pub async fn process_cluster_request(
    tracker: Arc<TorrentTracker>,
    _encoding: &ClusterEncoding,
    request: ClusterRequest,
) -> ClusterResponse {
    debug!(
        "[WEBSOCKET MASTER] Processing request {} from {}:{} - {:?}",
        request.request_id, request.client_ip, request.client_port, request.request_type
    );
    match request.request_type {
        RequestType::Announce => {
            process_announce(&tracker, &request).await
        }
        RequestType::Scrape => {
            process_scrape(&tracker, &request).await
        }
        RequestType::ApiCall { ref endpoint, ref method } => {
            process_api_call(&tracker, &request, endpoint, method).await
        }
        RequestType::UdpPacket => {
            process_udp_packet(&tracker, &request).await
        }
        RequestType::WtAnnounce => {
            process_webtorrent_announce(&tracker, &request).await
        }
        RequestType::WtScrape => {
            process_webtorrent_scrape(&tracker, &request).await
        }
        RequestType::WtOffer => {
            process_webtorrent_offer(&tracker, &request).await
        }
        RequestType::WtAnswer => {
            process_webtorrent_answer(&tracker, &request).await
        }
    }
}

pub async fn process_announce(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let query_string = match String::from_utf8(request.payload.clone()) {
        Ok(s) => s,
        Err(e) => {
            return ClusterResponse::error(
                request.request_id,
                format!("Invalid query string encoding: {}", e),
            );
        }
    };
    let query_map = match parse_query(Some(query_string)) {
        Ok(map) => map,
        Err(e) => {
            let error_response = ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode();
            return ClusterResponse::success(request.request_id, error_response);
        }
    };
    let announce = match tracker.validate_announce(request.client_ip, query_map).await {
        Ok(result) => result,
        Err(e) => {
            let error_response = ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode();
            return ClusterResponse::success(request.request_id, error_response);
        }
    };
    let tracker_config = &tracker.config.tracker_config;
    if tracker_config.whitelist_enabled && !tracker.check_whitelist(announce.info_hash) {
        let error_response = ben_map! {
            "failure reason" => ben_bytes!("unknown info_hash")
        }.encode();
        return ClusterResponse::success(request.request_id, error_response);
    }
    if tracker_config.blacklist_enabled && tracker.check_blacklist(announce.info_hash) {
        let error_response = ben_map! {
            "failure reason" => ben_bytes!("forbidden info_hash")
        }.encode();
        return ClusterResponse::success(request.request_id, error_response);
    }
    let (_torrent_peer, torrent_entry) = match tracker.handle_announce(tracker.clone(), announce.clone(), None).await {
        Ok(result) => result,
        Err(e) => {
            let error_response = ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode();
            return ClusterResponse::success(request.request_id, error_response);
        }
    };
    let stats = AnnounceResponseStats {
        interval: tracker_config.request_interval as i64,
        min_interval: tracker_config.request_interval_minimum as i64,
        complete: torrent_entry.seeds.len() as i64,
        incomplete: torrent_entry.peers.len() as i64,
        downloaded: torrent_entry.completed as i64,
    };
    let response_bytes = if announce.compact {
        build_compact_announce_response(
            tracker,
            &request.client_ip,
            &torrent_entry,
            &announce,
            &stats,
        )
    } else {
        build_extended_announce_response(
            tracker,
            &request.client_ip,
            &torrent_entry,
            &announce,
            &stats,
        )
    };
    ClusterResponse::success(request.request_id, response_bytes)
}

pub fn build_compact_announce_response(
    tracker: &Arc<TorrentTracker>,
    client_ip: &IpAddr,
    torrent_entry: &crate::tracker::structs::torrent_entry::TorrentEntry,
    announce: &crate::tracker::structs::announce_query_request::AnnounceQueryRequest,
    stats: &AnnounceResponseStats,
) -> Vec<u8> {
    let mut peers_list: Vec<u8> = Vec::with_capacity(72 * 6);
    let port_bytes = announce.port.to_be_bytes();
    match client_ip {
        IpAddr::V4(_) => {
            if announce.left != 0 {
                let seeds = tracker.get_peers(
                    &torrent_entry.seeds,
                    TorrentPeersType::IPv4,
                    Some(*client_ip),
                    72
                );
                for (_, torrent_peer) in seeds.iter() {
                    if let IpAddr::V4(ipv4) = torrent_peer.peer_addr.ip() {
                        let _ = peers_list.write(&ipv4.octets());
                        let _ = peers_list.write(&port_bytes);
                    }
                }
            }
            if peers_list.len() < 72 * 6 {
                let peers = tracker.get_peers(
                    &torrent_entry.peers,
                    TorrentPeersType::IPv4,
                    Some(*client_ip),
                    72
                );
                for (_, torrent_peer) in peers.iter() {
                    if peers_list.len() >= 72 * 6 {
                        break;
                    }
                    if let IpAddr::V4(ipv4) = torrent_peer.peer_addr.ip() {
                        let _ = peers_list.write(&ipv4.octets());
                        let _ = peers_list.write(&port_bytes);
                    }
                }
            }
            ben_map! {
                "interval" => ben_int!(stats.interval),
                "min interval" => ben_int!(stats.min_interval),
                "complete" => ben_int!(stats.complete),
                "incomplete" => ben_int!(stats.incomplete),
                "downloaded" => ben_int!(stats.downloaded),
                "peers" => ben_bytes!(peers_list)
            }.encode()
        }
        IpAddr::V6(_) => {
            if announce.left != 0 {
                let seeds = tracker.get_peers(
                    &torrent_entry.seeds,
                    TorrentPeersType::IPv6,
                    Some(*client_ip),
                    72
                );
                for (_, torrent_peer) in seeds.iter() {
                    if let IpAddr::V6(ipv6) = torrent_peer.peer_addr.ip() {
                        let _ = peers_list.write(&ipv6.octets());
                        let _ = peers_list.write(&port_bytes);
                    }
                }
            }
            if peers_list.len() < 72 * 18 {
                let peers = tracker.get_peers(
                    &torrent_entry.peers,
                    TorrentPeersType::IPv6,
                    Some(*client_ip),
                    72
                );
                for (_, torrent_peer) in peers.iter() {
                    if peers_list.len() >= 72 * 18 {
                        break;
                    }
                    if let IpAddr::V6(ipv6) = torrent_peer.peer_addr.ip() {
                        let _ = peers_list.write(&ipv6.octets());
                        let _ = peers_list.write(&port_bytes);
                    }
                }
            }
            ben_map! {
                "interval" => ben_int!(stats.interval),
                "min interval" => ben_int!(stats.min_interval),
                "complete" => ben_int!(stats.complete),
                "incomplete" => ben_int!(stats.incomplete),
                "downloaded" => ben_int!(stats.downloaded),
                "peers6" => ben_bytes!(peers_list)
            }.encode()
        }
    }
}

pub fn build_extended_announce_response(
    tracker: &Arc<TorrentTracker>,
    client_ip: &IpAddr,
    torrent_entry: &crate::tracker::structs::torrent_entry::TorrentEntry,
    announce: &crate::tracker::structs::announce_query_request::AnnounceQueryRequest,
    stats: &AnnounceResponseStats,
) -> Vec<u8> {
    let mut peers_list = ben_list!();
    let peers_list_mut = peers_list.list_mut().unwrap();
    match client_ip {
        IpAddr::V4(_) => {
            if announce.left != 0 {
                let seeds = tracker.get_peers(
                    &torrent_entry.seeds,
                    TorrentPeersType::IPv4,
                    Some(*client_ip),
                    72
                );
                for (peer_id, torrent_peer) in seeds.iter() {
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            if peers_list_mut.len() < 72 {
                let peers = tracker.get_peers(
                    &torrent_entry.peers,
                    TorrentPeersType::IPv4,
                    Some(*client_ip),
                    72
                );
                for (peer_id, torrent_peer) in peers.iter() {
                    if peers_list_mut.len() >= 72 {
                        break;
                    }
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            ben_map! {
                "interval" => ben_int!(stats.interval),
                "min interval" => ben_int!(stats.min_interval),
                "complete" => ben_int!(stats.complete),
                "incomplete" => ben_int!(stats.incomplete),
                "downloaded" => ben_int!(stats.downloaded),
                "peers" => peers_list
            }.encode()
        }
        IpAddr::V6(_) => {
            if announce.left != 0 {
                let seeds = tracker.get_peers(
                    &torrent_entry.seeds,
                    TorrentPeersType::IPv6,
                    Some(*client_ip),
                    72
                );
                for (peer_id, torrent_peer) in seeds.iter() {
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            if peers_list_mut.len() < 72 {
                let peers = tracker.get_peers(
                    &torrent_entry.peers,
                    TorrentPeersType::IPv6,
                    Some(*client_ip),
                    72
                );
                for (peer_id, torrent_peer) in peers.iter() {
                    if peers_list_mut.len() >= 72 {
                        break;
                    }
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            ben_map! {
                "interval" => ben_int!(stats.interval),
                "min interval" => ben_int!(stats.min_interval),
                "complete" => ben_int!(stats.complete),
                "incomplete" => ben_int!(stats.incomplete),
                "downloaded" => ben_int!(stats.downloaded),
                "peers6" => peers_list
            }.encode()
        }
    }
}

pub async fn process_scrape(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let query_string = match String::from_utf8(request.payload.clone()) {
        Ok(s) => s,
        Err(e) => {
            return ClusterResponse::error(
                request.request_id,
                format!("Invalid query string encoding: {}", e),
            );
        }
    };
    let query_map = match parse_query(Some(query_string)) {
        Ok(map) => map,
        Err(e) => {
            let error_response = ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode();
            return ClusterResponse::success(request.request_id, error_response);
        }
    };
    let scrape = match tracker.validate_scrape(query_map).await {
        Ok(result) => result,
        Err(e) => {
            let error_response = ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode();
            return ClusterResponse::success(request.request_id, error_response);
        }
    };
    let tracker_config = &tracker.config.tracker_config;
    let data_scrape = tracker.handle_scrape(tracker.clone(), scrape.clone()).await;
    let mut files_map = ben_map!();
    let files_map_mut = files_map.dict_mut().unwrap();
    for (info_hash, torrent_entry) in data_scrape.iter() {
        if tracker_config.whitelist_enabled && !tracker.check_whitelist(*info_hash) {
            continue;
        }
        if tracker_config.blacklist_enabled && tracker.check_blacklist(*info_hash) {
            continue;
        }
        files_map_mut.insert(
            Cow::from(info_hash.0.to_vec()),
            ben_map! {
                "complete" => ben_int!(torrent_entry.seeds.len() as i64),
                "downloaded" => ben_int!(torrent_entry.completed as i64),
                "incomplete" => ben_int!(torrent_entry.peers.len() as i64)
            }
        );
    }
    let response_bytes = ben_map! {
        "files" => files_map
    }.encode();
    ClusterResponse::success(request.request_id, response_bytes)
}

pub async fn process_api_call(
    _tracker: &Arc<TorrentTracker>,
    request: &ClusterRequest,
    endpoint: &str,
    method: &str,
) -> ClusterResponse {
    error!(
        "[WEBSOCKET MASTER] API calls through cluster not supported: {} {}",
        method, endpoint
    );
    let error_response = serde_json::json!({
        "error": "API calls through cluster not supported"
    }).to_string().into_bytes();
    ClusterResponse::success(request.request_id, error_response)
}

pub async fn process_udp_packet(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let remote_addr = SocketAddr::new(request.client_ip, request.client_port);
    let response = UdpServer::handle_packet(
        remote_addr,
        &request.payload,
        tracker.clone(),
        false,
    ).await;
    let estimated_size = response.estimated_size();
    let mut buffer = Vec::with_capacity(estimated_size);
    match response.write(&mut buffer) {
        Ok(_) => ClusterResponse::success(request.request_id, buffer),
        Err(e) => {
            error!("[WEBSOCKET MASTER] Failed to encode UDP response: {}", e);
            ClusterResponse::error(request.request_id, format!("Failed to encode UDP response: {}", e))
        }
    }
}

pub async fn process_webtorrent_announce(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let wt_announce: WtAnnounce = match serde_json::from_slice(&request.payload) {
        Ok(announce) => announce,
        Err(e) => {
            error!("[WEBSOCKET MASTER] Failed to parse WebTorrent announce: {}", e);
            let error_response = WtAnnounceResponse {
                info_hash: String::new(),
                complete: 0,
                incomplete: 0,
                peers: vec![],
                interval: tracker.config.tracker_config.request_interval as i64,
                failure_reason: Some(format!("Invalid announce: {}", e)),
                warning_message: None,
            };
            return ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&error_response).unwrap_or_default()
            );
        }
    };

    match handle_webtorrent_announce(tracker, wt_announce, request.client_ip).await {
        Ok(response) => {
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&response).unwrap_or_default()
            )
        }
        Err(e) => {
            error!("[WEBSOCKET MASTER] WebTorrent announce failed: {}", e);
            let error_response = WtAnnounceResponse {
                info_hash: String::new(),
                complete: 0,
                incomplete: 0,
                peers: vec![],
                interval: tracker.config.tracker_config.request_interval as i64,
                failure_reason: Some(format!("{}", e)),
                warning_message: None,
            };
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&error_response).unwrap_or_default()
            )
        }
    }
}

pub async fn process_webtorrent_scrape(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let wt_scrape: WtScrape = match serde_json::from_slice(&request.payload) {
        Ok(scrape) => scrape,
        Err(e) => {
            error!("[WEBSOCKET MASTER] Failed to parse WebTorrent scrape: {}", e);
            return ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&WtScrapeResponse { files: std::collections::HashMap::new() }).unwrap_or_default()
            );
        }
    };

    match handle_webtorrent_scrape(tracker, wt_scrape).await {
        Ok(response) => {
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&response).unwrap_or_default()
            )
        }
        Err(e) => {
            error!("[WEBSOCKET MASTER] WebTorrent scrape failed: {}", e);
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&WtScrapeResponse { files: std::collections::HashMap::new() }).unwrap_or_default()
            )
        }
    }
}

pub async fn process_webtorrent_offer(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let wt_offer: WtOffer = match serde_json::from_slice(&request.payload) {
        Ok(offer) => offer,
        Err(e) => {
            error!("[WEBSOCKET MASTER] Failed to parse WebTorrent offer: {}", e);
            let error_response = WtOfferResponse {
                info_hash: String::new(),
                peer_id: String::new(),
                offer_id: String::new(),
                error: Some(format!("Invalid offer: {}", e)),
            };
            return ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&error_response).unwrap_or_default()
            );
        }
    };

    match handle_webtorrent_offer(tracker, wt_offer, request.client_ip).await {
        Ok(response) => {
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&response).unwrap_or_default()
            )
        }
        Err(e) => {
            error!("[WEBSOCKET MASTER] WebTorrent offer failed: {}", e);
            let error_response = WtOfferResponse {
                info_hash: String::new(),
                peer_id: String::new(),
                offer_id: String::new(),
                error: Some(format!("{}", e)),
            };
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&error_response).unwrap_or_default()
            )
        }
    }
}

pub async fn process_webtorrent_answer(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    let wt_answer: WtAnswer = match serde_json::from_slice(&request.payload) {
        Ok(answer) => answer,
        Err(e) => {
            error!("[WEBSOCKET MASTER] Failed to parse WebTorrent answer: {}", e);
            let error_response = WtAnswerResponse {
                info_hash: String::new(),
                peer_id: String::new(),
                to_peer_id: String::new(),
                offer_id: String::new(),
                error: Some(format!("Invalid answer: {}", e)),
            };
            return ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&error_response).unwrap_or_default()
            );
        }
    };

    match handle_webtorrent_answer(tracker, wt_answer, request.client_ip).await {
        Ok(response) => {
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&response).unwrap_or_default()
            )
        }
        Err(e) => {
            error!("[WEBSOCKET MASTER] WebTorrent answer failed: {}", e);
            let error_response = WtAnswerResponse {
                info_hash: String::new(),
                peer_id: String::new(),
                to_peer_id: String::new(),
                offer_id: String::new(),
                error: Some(format!("{}", e)),
            };
            ClusterResponse::success(
                request.request_id,
                serde_json::to_vec(&error_response).unwrap_or_default()
            )
        }
    }
}

pub fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

pub async fn websocket_master_service(
    addr: SocketAddr,
    tracker: Arc<TorrentTracker>,
) -> (actix_web::dev::ServerHandle, impl std::future::Future<Output = Result<(), std::io::Error>>) {
    use actix_web::{web, App, HttpServer};
    use log::error;
    use std::fs::File;
    use std::io::BufReader;
    use std::process::exit;
    use std::time::Duration;

    let config = tracker.config.clone();
    let keep_alive = config.tracker_config.cluster_keep_alive;
    let request_timeout = config.tracker_config.cluster_request_timeout;
    let disconnect_timeout = config.tracker_config.cluster_disconnect_timeout;
    let worker_threads = config.tracker_config.cluster_threads as usize;
    let max_connections = config.tracker_config.cluster_max_connections as usize;
    let master_id = uuid::Uuid::new_v4().to_string();
    info!("[WEBSOCKET MASTER] Master UUID: {}", master_id);
    let service_data = Arc::new(WebSocketServiceData {
        tracker: tracker.clone(),
        config: config.clone(),
        master_id,
    });
    if config.tracker_config.cluster_ssl {
        info!("[WEBSOCKET MASTER] Starting WSS server on {}", addr);
        let ssl_key = &config.tracker_config.cluster_ssl_key;
        let ssl_cert = &config.tracker_config.cluster_ssl_cert;
        if ssl_key.is_empty() || ssl_cert.is_empty() {
            error!("[WEBSOCKET MASTER] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }
        let key_file = &mut BufReader::new(match File::open(ssl_key) {
            Ok(data) => data,
            Err(e) => {
                sentry::capture_error(&e);
                panic!("[WEBSOCKET MASTER] SSL key unreadable: {}", e);
            }
        });
        let certs_file = &mut BufReader::new(match File::open(ssl_cert) {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL cert unreadable: {}", e),
        });
        let tls_certs = match rustls_pemfile::certs(certs_file).collect::<Result<Vec<_>, _>>() {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL cert couldn't be extracted: {}", e),
        };
        let tls_key = match rustls_pemfile::pkcs8_private_keys(key_file).next().unwrap() {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL key couldn't be extracted: {}", e),
        };
        let tls_config = match rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
        {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL config couldn't be created: {}", e),
        };
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(service_data.clone()))
                .route("/cluster", web::get().to(websocket_handler))
        })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(request_timeout))
        .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
        .workers(worker_threads)
        .max_connections(max_connections)
        .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
        .unwrap()
        .disable_signals()
        .run();
        return (server.handle(), server);
    }
    info!("[WEBSOCKET MASTER] Starting WS server on {}", addr);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(service_data.clone()))
            .route("/cluster", web::get().to(websocket_handler))
    })
    .keep_alive(Duration::from_secs(keep_alive))
    .client_request_timeout(Duration::from_secs(request_timeout))
    .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
    .workers(worker_threads)
    .max_connections(max_connections)
    .bind((addr.ip(), addr.port()))
    .unwrap()
    .disable_signals()
    .run();
    (server.handle(), server)
}

pub async fn websocket_handler(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
    data: actix_web::web::Data<Arc<WebSocketServiceData>>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    use actix_web_actors::ws;
    use crate::websocket::structs::cluster_connection::ClusterConnection;

    let connection = ClusterConnection::new(data.get_ref().clone());
    ws::start(connection, &req, stream)
}

pub static SLAVE_CLIENT: once_cell::sync::Lazy<parking_lot::RwLock<SlaveClientState>> =
    once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(SlaveClientState::new()));

pub static SLAVE_SENDER: once_cell::sync::Lazy<SlaveSenderChannel> =
    once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(None));

pub fn is_connected() -> bool {
    SLAVE_CLIENT.read().connected
}

pub fn get_encoding() -> Option<ClusterEncoding> {
    SLAVE_CLIENT.read().encoding.clone()
}

pub async fn send_request(
    tracker: &Arc<TorrentTracker>,
    request: ClusterRequest,
) -> Result<ClusterResponse, ForwardError> {
    let (connected, encoding) = {
        let state = SLAVE_CLIENT.read();
        (state.connected, state.encoding.clone())
    };
    if !connected {
        return Err(ForwardError::NotConnected);
    }
    let encoding = match encoding {
        Some(e) => e,
        None => return Err(ForwardError::NotConnected),
    };
    let encoded = match encode(&encoding, &request) {
        Ok(data) => data,
        Err(e) => return Err(ForwardError::EncodingError(e.to_string())),
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    let request_id = request.request_id;
    {
        let mut state = SLAVE_CLIENT.write();
        state.pending_requests.insert(request_id, tx);
    }
    let send_result = {
        let sender_guard = SLAVE_SENDER.read();
        if let Some(sender) = sender_guard.as_ref() {
            sender.send(encoded).map_err(|_| ())
        } else {
            Err(())
        }
    };
    match send_result {
        Ok(_) => {
            tracker.update_stats(StatsEvent::WsRequestsSent, 1);
        }
        Err(_) => {
            let mut state = SLAVE_CLIENT.write();
            state.pending_requests.remove(&request_id);
            let sender_guard = SLAVE_SENDER.read();
            if sender_guard.is_none() {
                return Err(ForwardError::NotConnected);
            }
            return Err(ForwardError::ConnectionLost);
        }
    }
    let timeout_duration = std::time::Duration::from_secs(tracker.config.tracker_config.cluster_request_timeout);
    match tokio::time::timeout(timeout_duration, rx).await {
        Ok(Ok(response)) => {
            tracker.update_stats(StatsEvent::WsResponsesReceived, 1);
            Ok(response)
        }
        Ok(Err(_)) => {
            tracker.update_stats(StatsEvent::WsTimeouts, 1);
            Err(ForwardError::ConnectionLost)
        }
        Err(_) => {
            {
                let mut state = SLAVE_CLIENT.write();
                state.pending_requests.remove(&request_id);
            }
            tracker.update_stats(StatsEvent::WsTimeouts, 1);
            Err(ForwardError::Timeout)
        }
    }
}

pub async fn start_slave_client(tracker: Arc<TorrentTracker>) {
    let config = tracker.config.clone();
    let master_address = &config.tracker_config.cluster_master_address;
    let token = &config.tracker_config.cluster_token;
    let use_ssl = config.tracker_config.cluster_ssl;
    let reconnect_interval = config.tracker_config.cluster_reconnect_interval;
    let protocol = if use_ssl { "wss" } else { "ws" };
    let websocket_url = format!("{}://{}/cluster", protocol, master_address);
    let slave_id = uuid::Uuid::new_v4().to_string();
    info!("[WEBSOCKET SLAVE] Starting slave client, connecting to {}", websocket_url);
    info!("[WEBSOCKET SLAVE] Slave UUID: {}", slave_id);
    loop {
        match connect_to_master(
            &tracker,
            &websocket_url,
            token,
            &slave_id,
        ).await {
            Ok(()) => {
                info!("[WEBSOCKET SLAVE] Disconnected from master");
            }
            Err(e) => {
                error!("[WEBSOCKET SLAVE] Connection error: {}", e);
            }
        }
        {
            let mut state = SLAVE_CLIENT.write();
            state.connected = false;
            state.encoding = None;
            for (_, sender) in state.pending_requests.drain() {
                let _ = sender.send(ClusterResponse::error(0, "Connection lost".to_string()));
            }
        }
        {
            let mut sender_guard = SLAVE_SENDER.write();
            *sender_guard = None;
        }
        tracker.update_stats(StatsEvent::WsConnectionsActive, -1);
        tracker.update_stats(StatsEvent::WsReconnects, 1);
        info!(
            "[WEBSOCKET SLAVE] Reconnecting in {} seconds...",
            reconnect_interval
        );
        tokio::time::sleep(std::time::Duration::from_secs(reconnect_interval)).await;
    }
}

async fn connect_to_master(
    tracker: &Arc<TorrentTracker>,
    websocket_url: &str,
    token: &str,
    slave_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::websocket::structs::handshake::{
        HandshakeRequest,
        HandshakeResponse,
        CLUSTER_PROTOCOL_VERSION
    };
    use futures_util::{
        SinkExt,
        StreamExt
    };
    use tokio_tungstenite::{
        connect_async,
        tungstenite::Message
    };

    debug!("[WEBSOCKET SLAVE] Connecting to master: {}", websocket_url);
    let (ws_stream, _) = connect_async(websocket_url).await?;
    let (mut write, mut read) = ws_stream.split();
    info!("[WEBSOCKET SLAVE] Connected, sending handshake...");
    let handshake = HandshakeRequest::new(token.to_string(), slave_id.to_string());
    let handshake_data = serde_json::to_vec(&handshake)?;
    write.send(Message::Binary(handshake_data.into())).await?;
    let handshake_response: HandshakeResponse = match read.next().await {
        Some(Ok(Message::Binary(data))) => serde_json::from_slice(&data)?,
        Some(Ok(Message::Text(text))) => serde_json::from_str(&text)?,
        Some(Err(e)) => return Err(format!("WebSocket error during handshake: {}", e).into()),
        None => return Err("Connection closed during handshake".into()),
        _ => return Err("Unexpected message type during handshake".into()),
    };
    if !handshake_response.success {
        let error_msg = handshake_response.error.unwrap_or_else(|| "Unknown error".to_string());
        error!("[WEBSOCKET SLAVE] Handshake failed: {}", error_msg);
        tracker.update_stats(StatsEvent::WsAuthFailed, 1);
        return Err(format!("Handshake failed: {}", error_msg).into());
    }
    if handshake_response.version != CLUSTER_PROTOCOL_VERSION {
        warn!(
            "[WEBSOCKET SLAVE] Protocol version mismatch: master={}, slave={}",
            handshake_response.version, CLUSTER_PROTOCOL_VERSION
        );
    }
    let encoding = handshake_response.encoding.unwrap_or(ClusterEncoding::binary);
    let master_id = handshake_response.master_id.unwrap_or_else(|| "unknown".to_string());
    info!(
        "[WEBSOCKET SLAVE] Handshake successful, connected to master UUID: {}, using encoding: {:?}",
        master_id, encoding
    );
    tracker.update_stats(StatsEvent::WsAuthSuccess, 1);
    tracker.update_stats(StatsEvent::WsConnectionsActive, 1);
    {
        let mut state = SLAVE_CLIENT.write();
        state.connected = true;
        state.encoding = Some(encoding.clone());
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    {
        let mut sender_guard = SLAVE_SENDER.write();
        *sender_guard = Some(tx);
    }
    let write_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if write.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });
    let encoding_for_read = encoding.clone();
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                handle_response(&encoding_for_read, &data);
            }
            Ok(Message::Ping(data)) => {
                debug!("[WEBSOCKET SLAVE] Received ping");

                let _ = data;
            }
            Ok(Message::Pong(_)) => {
                debug!("[WEBSOCKET SLAVE] Received pong");
            }
            Ok(Message::Close(_)) => {
                info!("[WEBSOCKET SLAVE] Received close from master");
                break;
            }
            Err(e) => {
                error!("[WEBSOCKET SLAVE] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    write_handle.abort();
    Ok(())
}

fn handle_response(encoding: &ClusterEncoding, data: &[u8]) {
    let response: ClusterResponse = match decode(encoding, data) {
        Ok(r) => r,
        Err(e) => {
            error!("[WEBSOCKET SLAVE] Failed to decode response: {}", e);
            return;
        }
    };
    let mut state = SLAVE_CLIENT.write();
    if let Some(sender) = state.pending_requests.remove(&response.request_id) {
        let _ = sender.send(response);
    } else {
        warn!(
            "[WEBSOCKET SLAVE] Received response for unknown request: {}",
            response.request_id
        );
    }
}

pub async fn forward_request(
    tracker: &Arc<TorrentTracker>,
    protocol: crate::websocket::enums::protocol_type::ProtocolType,
    request_type: RequestType,
    client_ip: IpAddr,
    client_port: u16,
    payload: Vec<u8>,
) -> Result<ClusterResponse, ForwardError> {
    let request_id = {
        let mut state = SLAVE_CLIENT.write();
        state.next_request_id()
    };
    let request = ClusterRequest::new(
        request_id,
        protocol,
        request_type,
        client_ip,
        client_port,
        payload,
    );
    send_request(tracker, request).await
}

pub fn create_cluster_error_response(error: &ForwardError) -> Vec<u8> {
    let message = match error {
        ForwardError::NotConnected => "Cluster connection lost",
        ForwardError::Timeout => "Cluster timeout",
        ForwardError::MasterError(msg) => msg.as_str(),
        ForwardError::ConnectionLost => "Cluster connection lost",
        ForwardError::EncodingError(_) => "Cluster encoding error",
    };
    format!("d14:failure reason{}:{}e", message.len(), message).into_bytes()
}

pub fn create_cluster_error_response_json(error: &ForwardError) -> String {
    let message = match error {
        ForwardError::NotConnected => "Cluster connection lost",
        ForwardError::Timeout => "Cluster timeout",
        ForwardError::MasterError(msg) => msg.as_str(),
        ForwardError::ConnectionLost => "Cluster connection lost",
        ForwardError::EncodingError(_) => "Cluster encoding error",
    };
    serde_json::json!({
        "failure_reason": message
    }).to_string()
}