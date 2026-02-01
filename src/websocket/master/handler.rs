

use std::borrow::Cow;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use bip_bencode::{ben_bytes, ben_int, ben_list, ben_map, BMutAccess};
use log::{debug, error};

use crate::common::common::parse_query;
use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::udp_server::UdpServer;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::cluster_response::ClusterResponse;

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
    }
}

async fn process_announce(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    
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

    
    let request_interval = tracker_config.request_interval as i64;
    let request_interval_minimum = tracker_config.request_interval_minimum as i64;
    let seeds_count = torrent_entry.seeds.len() as i64;
    let peers_count = torrent_entry.peers.len() as i64;
    let completed_count = torrent_entry.completed as i64;

    let response_bytes = if announce.compact {
        
        build_compact_announce_response(
            tracker,
            &request.client_ip,
            &torrent_entry,
            &announce,
            request_interval,
            request_interval_minimum,
            seeds_count,
            peers_count,
            completed_count,
        )
    } else {
        
        build_extended_announce_response(
            tracker,
            &request.client_ip,
            &torrent_entry,
            &announce,
            request_interval,
            request_interval_minimum,
            seeds_count,
            peers_count,
            completed_count,
        )
    };

    ClusterResponse::success(request.request_id, response_bytes)
}

fn build_compact_announce_response(
    tracker: &Arc<TorrentTracker>,
    client_ip: &IpAddr,
    torrent_entry: &crate::tracker::structs::torrent_entry::TorrentEntry,
    announce: &crate::tracker::structs::announce_query_request::AnnounceQueryRequest,
    interval: i64,
    min_interval: i64,
    complete: i64,
    incomplete: i64,
    downloaded: i64,
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
                "interval" => ben_int!(interval),
                "min interval" => ben_int!(min_interval),
                "complete" => ben_int!(complete),
                "incomplete" => ben_int!(incomplete),
                "downloaded" => ben_int!(downloaded),
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
                "interval" => ben_int!(interval),
                "min interval" => ben_int!(min_interval),
                "complete" => ben_int!(complete),
                "incomplete" => ben_int!(incomplete),
                "downloaded" => ben_int!(downloaded),
                "peers6" => ben_bytes!(peers_list)
            }.encode()
        }
    }
}

fn build_extended_announce_response(
    tracker: &Arc<TorrentTracker>,
    client_ip: &IpAddr,
    torrent_entry: &crate::tracker::structs::torrent_entry::TorrentEntry,
    announce: &crate::tracker::structs::announce_query_request::AnnounceQueryRequest,
    interval: i64,
    min_interval: i64,
    complete: i64,
    incomplete: i64,
    downloaded: i64,
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
                "interval" => ben_int!(interval),
                "min interval" => ben_int!(min_interval),
                "complete" => ben_int!(complete),
                "incomplete" => ben_int!(incomplete),
                "downloaded" => ben_int!(downloaded),
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
                "interval" => ben_int!(interval),
                "min interval" => ben_int!(min_interval),
                "complete" => ben_int!(complete),
                "incomplete" => ben_int!(incomplete),
                "downloaded" => ben_int!(downloaded),
                "peers6" => peers_list
            }.encode()
        }
    }
}

async fn process_scrape(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    
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

async fn process_api_call(
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

async fn process_udp_packet(tracker: &Arc<TorrentTracker>, request: &ClusterRequest) -> ClusterResponse {
    
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
