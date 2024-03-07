use log::debug;
use async_std::sync::Arc;
use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::time::SystemTime;

use crate::common::{AnnounceEvent, AnnounceQueryRequest, CustomError, InfoHash, NumberOfBytes, PeerId, ScrapeQueryRequest, TorrentPeer, UserId};
use crate::config::Configuration;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::torrents::{TorrentEntry, TorrentEntryItem};

pub async fn validate_announce(config: Arc<Configuration>, remote_addr: IpAddr, query: HashMap<String, Vec<Vec<u8>>>) -> Result<AnnounceQueryRequest, CustomError>
{
    // Validate info_hash
    let info_hash: Vec<Vec<u8>> = match query.get("info_hash") {
        None => {
            return Err(CustomError::new("missing info_hash"));
        }
        Some(result) => {
            if result.is_empty() {
                return Err(CustomError::new("no info_hash given"));
            }
            if let Some(result_array) = result.get(0) {
                if result_array.len() != 20 {
                    return Err(CustomError::new("invalid info_hash size"));
                }
                result.clone()
            } else {
                return Err(CustomError::new("no info_hash given"));
            }
        }
    };

    // Validate peer_id
    let peer_id: Vec<Vec<u8>> = match query.get("peer_id") {
        None => {
            return Err(CustomError::new("missing peer_id"));
        }
        Some(result) => {
            if result.is_empty() {
                return Err(CustomError::new("no peer_id given"));
            }
            if let Some(result_array) = result.get(0) {
                if result_array.len() != 20 {
                    return Err(CustomError::new("invalid peer_id size"));
                }
                result.clone()
            } else {
                return Err(CustomError::new("no peer_id given"));
            }
        }
    };

    // Validate port
    let port_integer = match query.get("port") {
        None => {
            return Err(CustomError::new("missing port"));
        }
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let port = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid port"))
                };
                match port.parse::<u16>() {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("missing or invalid port"))
                }
            } else {
                return Err(CustomError::new("missing port"));
            }
        }
    };

    // Validate uploaded
    let uploaded_integer = match query.get("uploaded") {
        None => {
            return Err(CustomError::new("missing uploaded"));
        }
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let uploaded = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid uploaded"))
                };
                match uploaded.parse::<u64>() {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("missing or invalid uploaded"))
                }
            } else {
                return Err(CustomError::new("missing uploaded"));
            }
        }
    };

    // Validate downloaded
    let downloaded_integer = match query.get("downloaded") {
        None => {
            return Err(CustomError::new("missing downloaded"));
        }
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let downloaded = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid downloaded"))
                };
                match downloaded.parse::<u64>() {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("missing or invalid downloaded"))
                }
            } else {
                return Err(CustomError::new("missing downloaded"));
            }
        }
    };

    // Validate left
    let left_integer = match query.get("left") {
        None => {
            return Err(CustomError::new("missing left"));
        }
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let left = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid left"))
                };
                match left.parse::<u64>() {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("missing or invalid left"))
                }
            } else {
                return Err(CustomError::new("missing left"));
            }
        }
    };

    // Validate compact
    let mut compact_bool = false;
    match query.get("compact") {
        None => {}
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let compact = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid compact"))
                };
                let compact_integer = match compact.parse::<u8>() {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("missing or invalid compact"))
                };
                if compact_integer == 1 {
                    compact_bool = true;
                }
            }
        }
    }

    // Validate event
    let mut event_integer: AnnounceEvent = AnnounceEvent::Started;
    match query.get("event") {
        None => {}
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let event = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid event"))
                };
                match event.as_str().to_lowercase().as_str() {
                    "started" => {
                        event_integer = AnnounceEvent::Started;
                    }
                    "stopped" => {
                        event_integer = AnnounceEvent::Stopped;
                    }
                    "completed" => {
                        event_integer = AnnounceEvent::Completed;
                    }
                    _ => {
                        event_integer = AnnounceEvent::Started;
                    }
                }
            } else {
                event_integer = AnnounceEvent::Started;
            }
        }
    }

    // Validate no_peer_id
    let mut no_peer_id_bool = false;
    match query.get("no_peer_id") {
        None => {}
        Some(_) => {
            no_peer_id_bool = true;
        }
    }

    // Validate numwant
    let mut numwant_integer = config.peers_returned.unwrap();
    match query.get("numwant") {
        None => {}
        Some(result) => {
            if let Some(result_array) = result.get(0) {
                let numwant = match String::from_utf8(result_array.to_vec()) {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("invalid numwant"))
                };
                numwant_integer = match numwant.parse::<u64>() {
                    Ok(v) => v,
                    Err(_) => return Err(CustomError::new("missing or invalid numwant"))
                };
                if numwant_integer == 0 || numwant_integer > config.peers_returned.unwrap() {
                    numwant_integer = config.peers_returned.unwrap();
                }
            }
        }
    }

    let announce_data = AnnounceQueryRequest {
        info_hash: InfoHash::from(&info_hash[0] as &[u8]),
        peer_id: PeerId::from(&peer_id[0] as &[u8]),
        port: port_integer,
        uploaded: uploaded_integer,
        downloaded: downloaded_integer,
        left: left_integer,
        compact: compact_bool,
        no_peer_id: no_peer_id_bool,
        event: event_integer,
        remote_addr,
        numwant: numwant_integer,
    };

    Ok(announce_data)
}

pub async fn handle_announce(data: Arc<TorrentTracker>, announce_query: AnnounceQueryRequest, user_key: Option<UserId>) -> Result<(TorrentPeer, TorrentEntry), CustomError>
{
    if (data.get_torrent(announce_query.info_hash).await).is_none() {
        if data.config.persistence {
            data.add_torrent(announce_query.info_hash, TorrentEntryItem::new(), true).await;
        } else {
            data.add_torrent(announce_query.info_hash, TorrentEntryItem::new(), false).await;
        }
    }

    let mut torrent_peer = TorrentPeer {
        peer_id: announce_query.peer_id,
        peer_addr: SocketAddr::new(announce_query.remote_addr, announce_query.port),
        peer_offer_id: None,
        peer_offer: None,
        updated: std::time::Instant::now(),
        uploaded: NumberOfBytes(announce_query.uploaded as i64),
        downloaded: NumberOfBytes(announce_query.downloaded as i64),
        left: NumberOfBytes(announce_query.left as i64),
        event: AnnounceEvent::None,
    };

    match announce_query.event {
        AnnounceEvent::Started => {
            torrent_peer.event = AnnounceEvent::Started;
            debug!("[HANDLE ANNOUNCE] Adding to infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id.to_string());
            let torrent_entry = data.add_peer(
                announce_query.info_hash,
                announce_query.peer_id,
                torrent_peer.clone(),
                false,
                data.config.persistence,
            ).await;
            let peer_list = parse_ip_format(
                torrent_entry.peers.clone(),
                data.config.clone(),
                announce_query.remote_addr,
                torrent_peer.peer_addr,
            );
            if data.config.users && user_key.is_some(){
                if let Some(mut user) = data.get_user(user_key.unwrap()).await {
                    user.updated = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    let mut torrents_active = user.torrents_active.clone();
                    torrents_active.insert(announce_query.info_hash, std::time::Instant::now());
                    user.torrents_active = torrents_active;
                    data.add_user(user_key.unwrap(), user).await;
                }
            }
            Ok((torrent_peer, TorrentEntry {
                peers: peer_list,
                completed: torrent_entry.completed,
                seeders: torrent_entry.seeders,
                leechers: torrent_entry.leechers,
            }))
        }
        AnnounceEvent::Stopped => {
            torrent_peer.event = AnnounceEvent::Stopped;
            debug!("[HANDLE ANNOUNCE] Removing from infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id.to_string());
            let torrent = data.remove_peer(announce_query.info_hash, announce_query.peer_id, data.config.persistence).await;
            if data.config.users && user_key.is_some(){
                if let Some(mut user) = data.get_user(user_key.unwrap()).await {
                    user.uploaded += announce_query.uploaded;
                    user.downloaded += announce_query.downloaded;
                    user.updated = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    let mut torrents_active = user.torrents_active.clone();
                    torrents_active.remove(&announce_query.info_hash);
                    user.torrents_active = torrents_active;
                    data.add_user(user_key.unwrap(), user.clone()).await;
                    data.add_users_update(user_key.unwrap(), user).await;
                }
            }
            Ok((torrent_peer, TorrentEntry {
                peers: BTreeMap::new(),
                completed: torrent.completed,
                seeders: torrent.seeders,
                leechers: torrent.leechers,
            }))
        }
        AnnounceEvent::Completed => {
            torrent_peer.event = AnnounceEvent::Completed;
            debug!("[HANDLE ANNOUNCE] Adding to infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id.to_string());
            let torrent_entry = data.add_peer(
                announce_query.info_hash,
                announce_query.peer_id,
                torrent_peer.clone(),
                true,
                data.config.persistence,
            ).await;
            let peer_list = parse_ip_format(
                torrent_entry.peers.clone(),
                data.config.clone(),
                announce_query.remote_addr,
                torrent_peer.peer_addr,
            );
            if data.config.users && user_key.is_some(){
                if let Some(mut user) = data.get_user(user_key.unwrap()).await {
                    user.completed += 1;
                    user.updated = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    data.add_user(user_key.unwrap(), user).await;
                }
            }
            Ok((torrent_peer, TorrentEntry {
                peers: peer_list,
                completed: torrent_entry.completed,
                seeders: torrent_entry.seeders,
                leechers: torrent_entry.leechers,
            }))
        }
        AnnounceEvent::None => {
            debug!("[HANDLE ANNOUNCE] Adding to infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id.to_string());
            let torrent_entry = data.add_peer(
                announce_query.info_hash,
                announce_query.peer_id,
                torrent_peer.clone(),
                false,
                data.config.persistence,
            ).await;
            let peer_list = parse_ip_format(
                torrent_entry.peers.clone(),
                data.config.clone(),
                announce_query.remote_addr,
                torrent_peer.peer_addr,
            );
            if data.config.users && user_key.is_some(){
                if let Some(mut user) = data.get_user(user_key.unwrap()).await {
                    user.updated = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    let mut torrents_active = user.torrents_active.clone();
                    torrents_active.insert(announce_query.info_hash, std::time::Instant::now());
                    user.torrents_active = torrents_active;
                    data.add_user(user_key.unwrap(), user).await;
                }
            }
            Ok((torrent_peer, TorrentEntry {
                peers: peer_list,
                completed: torrent_entry.completed,
                seeders: torrent_entry.seeders,
                leechers: torrent_entry.leechers,
            }))
        }
    }
}

pub async fn validate_scrape(_config: Arc<Configuration>, _remote_addr: IpAddr, query: HashMap<String, Vec<Vec<u8>>>) -> Result<ScrapeQueryRequest, CustomError>
{
    // Validate info_hash
    let mut info_hash: Vec<InfoHash> = Vec::new();
    match query.get("info_hash") {
        None => {
            Err(CustomError::new("missing info_hash"))
        }
        Some(result) => {
            if result.is_empty() {
                return Err(CustomError::new("no info_hash given"));
            }

            for hash in result.iter() {
                if hash.len() != 20 {
                    return Err(CustomError::new("an invalid info_hash was given"));
                }
                info_hash.push(InfoHash::from(hash as &[u8]));
            }

            let scrape_data = ScrapeQueryRequest {
                info_hash
            };

            Ok(scrape_data)
        }
    }
}

pub async fn handle_scrape(data: Arc<TorrentTracker>, scrape_query: ScrapeQueryRequest) -> BTreeMap<InfoHash, TorrentEntry>
{
    // We generate the output and return it, even if it's empty...
    let mut return_data = BTreeMap::new();
    for hash in scrape_query.info_hash.iter() {
        match data.get_torrent(*hash).await {
            None => {
                return_data.insert(*hash, TorrentEntry {
                    peers: BTreeMap::new(),
                    completed: 0,
                    seeders: 0,
                    leechers: 0,
                });
            }
            Some(result) => {
                return_data.insert(*hash, TorrentEntry {
                    peers: BTreeMap::new(),
                    completed: result.completed,
                    seeders: result.seeders,
                    leechers: result.leechers,
                });
            }
        }
    }

    return_data
}

fn parse_ip_format(peers: BTreeMap<PeerId, TorrentPeer>, config: Arc<Configuration>, remote_addr: IpAddr, peer_addr: SocketAddr) -> BTreeMap<PeerId, TorrentPeer> {
    let mut peers_parsed = 0u64;
    let mut peer_list = BTreeMap::new();
    for (peer_id, torrent_peer) in peers.iter() {
        if peers_parsed == config.peers_returned.unwrap() {
            break;
        }
        if remote_addr.is_ipv4() && peer_addr.is_ipv4() {
            peer_list.insert(*peer_id, torrent_peer.clone());
            peers_parsed += 1;
        }
        if remote_addr.is_ipv6() && peer_addr.is_ipv6() {
            peer_list.insert(*peer_id, torrent_peer.clone());
            peers_parsed += 1;
        }
    }
    peer_list
}