use crate::common::common::QueryValues;
use crate::common::structs::custom_error::CustomError;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::announce_query_request::AnnounceQueryRequest;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::scrape_query_request::ScrapeQueryRequest;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_id::UserId;
use log::debug;
use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::SystemTime;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn validate_announce(&self, remote_addr: IpAddr, query: HashMap<String, QueryValues>) -> Result<AnnounceQueryRequest, CustomError>
    {
        let now = std::time::Instant::now();

        #[inline]
        fn get_required_bytes<'a>(query: &'a HashMap<String, QueryValues>, field: &str, expected_len: Option<usize>) -> Result<&'a [u8], CustomError> {
            let value = query.get(field)
                .ok_or_else(|| CustomError::new(&format!("missing {field}")))?
                .first()
                .ok_or_else(|| CustomError::new(&format!("no {field} given")))?;
            if let Some(len) = expected_len && value.len() != len {
                return Err(CustomError::new(&format!("invalid {field} size")));
            }
            Ok(value.as_slice())
        }

        #[inline]
        fn parse_integer<T: std::str::FromStr>(query: &HashMap<String, QueryValues>, field: &str) -> Result<T, CustomError> {
            let bytes = get_required_bytes(query, field, None)?;
            let str_value = std::str::from_utf8(bytes)
                .map_err(|_| CustomError::new(&format!("invalid {field}")))?;
            str_value.parse::<T>()
                .map_err(|_| CustomError::new(&format!("missing or invalid {field}")))
        }

        let info_hash_bytes = get_required_bytes(&query, "info_hash", Some(20))?;
        let peer_id_bytes = get_required_bytes(&query, "peer_id", Some(20))?;
        let port_integer = parse_integer::<u16>(&query, "port")?;
        let info_hash = InfoHash::from(info_hash_bytes);
        let peer_id = PeerId::from(peer_id_bytes);
        let uploaded_integer = parse_integer::<u64>(&query, "uploaded").unwrap_or(0);
        let downloaded_integer = parse_integer::<u64>(&query, "downloaded").unwrap_or(0);
        let left_integer = parse_integer::<u64>(&query, "left").unwrap_or(0);
        let compact_bool = query.get("compact")
            .and_then(|v| v.first())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u8>().ok())
            .map(|v| v == 1)
            .unwrap_or(false);
        let event_integer = query.get("event")
            .and_then(|v| v.first())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|s| match s.to_lowercase().as_str() {
                "stopped" => AnnounceEvent::Stopped,
                "completed" => AnnounceEvent::Completed,
                _ => AnnounceEvent::Started,
            })
            .unwrap_or(AnnounceEvent::Started);
        let no_peer_id_bool = query.contains_key("no_peer_id");
        let numwant_integer = query.get("numwant")
            .and_then(|v| v.first())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(|v| if v == 0 || v > 72 { 72 } else { v })
            .unwrap_or(72);
        let elapsed = now.elapsed();
        debug!("[PERF] Announce validation took: {:?}", elapsed);
        Ok(AnnounceQueryRequest {
            info_hash,
            peer_id,
            port: port_integer,
            uploaded: uploaded_integer,
            downloaded: downloaded_integer,
            left: left_integer,
            compact: compact_bool,
            no_peer_id: no_peer_id_bool,
            event: event_integer,
            remote_addr,
            numwant: numwant_integer,
        })
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_announce(&self, data: Arc<TorrentTracker>, announce_query: AnnounceQueryRequest, user_key: Option<UserId>) -> Result<(TorrentPeer, TorrentEntry), CustomError>
    {
        let now = std::time::Instant::now();
        let mut torrent_peer = TorrentPeer {
            peer_id: announce_query.peer_id,
            peer_addr: SocketAddr::new(announce_query.remote_addr, announce_query.port),
            updated: std::time::Instant::now(),
            uploaded: NumberOfBytes(announce_query.uploaded as i64),
            downloaded: NumberOfBytes(announce_query.downloaded as i64),
            left: NumberOfBytes(announce_query.left as i64),
            event: AnnounceEvent::None,
        };
        let is_persistent = data.config.database.persistent;
        let users_enabled = data.config.tracker_config.users_enabled;
        match announce_query.event {
            AnnounceEvent::Started | AnnounceEvent::None => {
                torrent_peer.event = AnnounceEvent::Started;
                debug!("[HANDLE ANNOUNCE] Adding to infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id);
                let torrent_entry = data.add_torrent_peer(
                    announce_query.info_hash,
                    announce_query.peer_id,
                    torrent_peer.clone(),
                    false
                );
                if is_persistent {
                    let _ = data.add_torrent_update(
                        announce_query.info_hash,
                        torrent_entry.1.clone(),
                        UpdatesAction::Add
                    );
                }
                if users_enabled && let Some(user_id) = user_key && let Some(mut user) = data.get_user(user_id) {
                    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    user.updated = now;
                    user.torrents_active.insert(announce_query.info_hash, now);
                    data.add_user(user_id, user.clone());
                    if is_persistent {
                        data.add_user_update(user_id, user, UpdatesAction::Add);
                    }
                }
                let elapsed = now.elapsed();
                debug!("[PERF] Announce Started handling took: {elapsed:?}");
                Ok((torrent_peer, TorrentEntry {
                    seeds: torrent_entry.1.seeds,
                    peers: torrent_entry.1.peers,
                    completed: torrent_entry.1.completed,
                    updated: torrent_entry.1.updated
                }))
            }
            AnnounceEvent::Stopped => {
                torrent_peer.event = AnnounceEvent::Stopped;
                debug!("[HANDLE ANNOUNCE] Removing from infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id);
                let torrent_entry = match data.remove_torrent_peer(
                    announce_query.info_hash,
                    announce_query.peer_id,
                    is_persistent,
                    false
                ) {
                    (Some(_), Some(new_torrent)) => {
                        if users_enabled && let Some(user_id) = user_key && let Some(mut user) = data.get_user(user_id) {
                            user.uploaded += announce_query.uploaded;
                            user.downloaded += announce_query.downloaded;
                            user.updated = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                            user.torrents_active.remove(&announce_query.info_hash);
                            data.add_user(user_id, user.clone());
                            if is_persistent {
                                data.add_user_update(user_id, user, UpdatesAction::Add);
                            }
                        }
                        new_torrent
                    }
                    _ => TorrentEntry::new()
                };
                if is_persistent {
                    let _ = data.add_torrent_update(
                        announce_query.info_hash,
                        torrent_entry.clone(),
                        UpdatesAction::Add
                    );
                }
                let elapsed = now.elapsed();
                debug!("[PERF] Announce Stopped handling took: {elapsed:?}");
                Ok((torrent_peer, torrent_entry))
            }
            AnnounceEvent::Completed => {
                torrent_peer.event = AnnounceEvent::Completed;
                debug!("[HANDLE ANNOUNCE] Adding to infohash {} peerid {}", announce_query.info_hash, announce_query.peer_id);
                let torrent_entry = data.add_torrent_peer(
                    announce_query.info_hash,
                    announce_query.peer_id,
                    torrent_peer.clone(),
                    true
                );
                if is_persistent {
                    let _ = data.add_torrent_update(
                        announce_query.info_hash,
                        torrent_entry.1.clone(),
                        UpdatesAction::Add
                    );
                }
                if users_enabled && let Some(user_id) = user_key && let Some(mut user) = data.get_user(user_id) {
                    user.completed += 1;
                    user.updated = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    data.add_user(user_id, user.clone());
                    if is_persistent {
                        data.add_user_update(user_id, user, UpdatesAction::Add);
                    }
                }
                let elapsed = now.elapsed();
                debug!("[PERF] Announce Completed handling took: {elapsed:?}");
                Ok((torrent_peer, torrent_entry.1))
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn validate_scrape(&self, query: HashMap<String, QueryValues>) -> Result<ScrapeQueryRequest, CustomError>
    {
        let now = std::time::Instant::now();
        match query.get("info_hash") {
            None => Err(CustomError::new("missing info_hash")),
            Some(result) => {
                if result.is_empty() {
                    return Err(CustomError::new("no info_hash given"));
                }
                let mut info_hash_vec = Vec::with_capacity(result.len());
                for hash in result.iter() {
                    if hash.len() != 20 {
                        return Err(CustomError::new("an invalid info_hash was given"));
                    }
                    info_hash_vec.push(InfoHash::from(hash.as_slice()));
                }
                let elapsed = now.elapsed();
                debug!("[PERF] Scrape validation took: {elapsed:?}");
                Ok(ScrapeQueryRequest { info_hash: info_hash_vec })
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn handle_scrape(&self, data: Arc<TorrentTracker>, scrape_query: ScrapeQueryRequest) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let now = std::time::Instant::now();
        let result = scrape_query.info_hash.iter()
            .map(|&info_hash| {
                let entry = data.get_torrent(info_hash).unwrap_or_default();
                (info_hash, entry)
            })
            .collect();
        let elapsed = now.elapsed();
        debug!("[PERF] Scrape handling took: {:?}", elapsed);
        result
    }
}