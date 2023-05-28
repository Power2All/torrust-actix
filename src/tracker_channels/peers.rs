use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use log::{debug, info};
use serde_json::{json, Value};

use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;
use crate::tracker_channels::torrents::TorrentEntry;

impl TorrentTracker {
    pub fn channel_peers_init(&self)
    {
        let (_channel_left, channel_right) = self.peers_channel.clone();
        tokio::spawn(async move {
            let mut peers: BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>> = BTreeMap::new();

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_peers_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, _channel_right) = self.peers_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub async fn add_peer(&self, info_hash: InfoHash, peer_id: PeerId, peer_entry: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let mut added_seeder = false;
        let mut added_leecher = false;
        let mut removed_seeder = false;
        let mut removed_leecher = false;
        let mut completed_applied = false;

        let torrents_arc = self.map_torrents.clone();
        let torrents_lock = torrents_arc.read().await;
        let torrent_input = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.read().await;
                let peer = peers_lock.get(&info_hash).cloned();
                drop(peers_lock);

                let mut peers = match peer {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers }
                };

                match peers.get(&peer_id).cloned() {
                    None => {
                        if peer_entry.left == NumberOfBytes(0) {
                            data_torrent.seeders += 1;
                            added_seeder = true;
                            if completed {
                                data_torrent.completed += 1;
                                completed_applied = true;
                            }
                        } else {
                            data_torrent.leechers += 1;
                            added_leecher = true;
                        }
                        let _ = peers.insert(peer_id, peer_entry);
                    }
                    Some(data_peer) => {
                        if data_peer.left == NumberOfBytes(0) && peer_entry.left != NumberOfBytes(0) {
                            data_torrent.seeders -= 1;
                            data_torrent.leechers += 1;
                            removed_seeder = true;
                            added_leecher = true;
                        } else if data_peer.left != NumberOfBytes(0) && peer_entry.left == NumberOfBytes(0) {
                            data_torrent.seeders += 1;
                            data_torrent.leechers -= 1;
                            added_seeder = true;
                            removed_leecher = true;
                            if completed {
                                data_torrent.completed += 1;
                                completed_applied = true;
                            }
                        }
                        let _ = peers.insert(peer_id, peer_entry);
                    }
                };

                let torrents_arc = self.map_torrents.clone();
                let mut torrents_lock = torrents_arc.write().await;
                torrents_lock.insert(info_hash, data_torrent.clone());
                drop(torrents_lock);

                let peers_arc = self.map_peers.clone();
                let mut peers_lock = peers_arc.write().await;
                peers_lock.insert(info_hash, peers.clone());
                drop(peers_lock);

                TorrentEntry {
                    peers,
                    completed: data_torrent.completed,
                    seeders: data_torrent.seeders,
                    leechers: data_torrent.leechers,
                }
            }
        };

        if persistent && completed {
            self.add_update(
                info_hash,
                torrent.completed,
            ).await;
        }

        if added_seeder { self.update_stats(StatsEvent::Seeds, 1).await; }
        if added_leecher { self.update_stats(StatsEvent::Peers, 1).await; }
        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
        if completed_applied { self.update_stats(StatsEvent::Completed, 1).await; }

        torrent
    }

    pub async fn remove_peer(&self, info_hash: InfoHash, peer_id: PeerId, _persistent: bool) -> TorrentEntry
    {
        let mut removed_seeder = false;
        let mut removed_leecher = false;

        let torrents_arc = self.map_torrents.clone();
        let torrents_lock = torrents_arc.read().await;
        let torrent_input = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.read().await;
                let peer = peers_lock.get(&info_hash).cloned();
                drop(peers_lock);

                let mut peers = match peer {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers }
                };
                let peer_option = peers.get(&peer_id);
                if peer_option.is_some() {
                    let peer = *peer_option.unwrap();
                    if peer.left == NumberOfBytes(0) {
                        peers.remove(&peer_id);
                        data_torrent.seeders -= 1;
                        removed_seeder = true;
                    } else {
                        peers.remove(&peer_id);
                        data_torrent.leechers -= 1;
                        removed_leecher = true;
                    }
                }

                let torrents_arc = self.map_torrents.clone();
                let mut torrents_lock = torrents_arc.write().await;
                torrents_lock.insert(info_hash, data_torrent.clone());
                drop(torrents_lock);

                if peers.is_empty() {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.write().await;
                    peers_lock.remove(&info_hash);
                    drop(peers_lock);
                } else {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.write().await;
                    peers_lock.insert(info_hash, peers.clone());
                    drop(peers_lock);
                }

                TorrentEntry {
                    peers,
                    completed: data_torrent.completed,
                    seeders: data_torrent.seeders,
                    leechers: data_torrent.leechers,
                }
            }
        };

        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }

        torrent
    }

    pub async fn remove_peers(&self, peers: Vec<(InfoHash, PeerId)>, _persistent: bool) -> HashMap<InfoHash, TorrentEntry>
    {
        let mut removed_seeder = 0i64;
        let mut removed_leecher = 0i64;
        let mut return_torrententries = HashMap::new();

        for (info_hash, peer_id) in peers.iter() {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.read().await;
            let torrent = torrents_lock.get(info_hash).cloned();
            drop(torrents_lock);

            return_torrententries.insert(*info_hash, match torrent {
                None => { TorrentEntry::new() }
                Some(mut data_torrent) => {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.read().await;
                    let peer = peers_lock.get(info_hash).cloned();
                    drop(peers_lock);

                    let mut peers = match peer {
                        None => { BTreeMap::new() }
                        Some(data_peers) => { data_peers }
                    };

                    let peer_option = peers.get(peer_id);
                    if peer_option.is_some() {
                        let peer = *peer_option.unwrap();
                        if peer.left == NumberOfBytes(0) {
                            peers.remove(peer_id);
                            data_torrent.seeders -= 1;
                            removed_seeder -= 1;
                        } else {
                            peers.remove(peer_id);
                            data_torrent.leechers -= 1;
                            removed_leecher -= 1;
                        }
                    }

                    let torrents_arc = self.map_torrents.clone();
                    let mut torrents_lock = torrents_arc.write().await;
                    torrents_lock.insert(*info_hash, data_torrent.clone());
                    drop(torrents_lock);

                    if peers.is_empty() {
                        let peers_arc = self.map_peers.clone();
                        let mut peers_lock = peers_arc.write().await;
                        peers_lock.remove(info_hash);
                        drop(peers_lock);
                    } else {
                        let peers_arc = self.map_peers.clone();
                        let mut peers_lock = peers_arc.write().await;
                        peers_lock.insert(*info_hash, peers.clone());
                        drop(peers_lock);
                    }

                    TorrentEntry {
                        peers,
                        completed: data_torrent.completed,
                        seeders: data_torrent.seeders,
                        leechers: data_torrent.leechers,
                    }
                }
            });
        }

        if removed_seeder != 0 { self.update_stats(StatsEvent::Seeds, removed_seeder).await; }
        if removed_leecher != 0 { self.update_stats(StatsEvent::Peers, removed_leecher).await; }

        return_torrententries
    }

    pub async fn clean_peers(&self, peer_timeout: Duration)
    {
        // Cleaning up peers in chunks, to prevent slow behavior.
        let mut start: usize = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        let mut removed_peers = 0u64;

        loop {
            info!("[PEERS] Scanning peers {} to {}", start, (start + size));

            let peers_arc = self.map_peers.clone();
            let peers_lock = peers_arc.read().await;
            let mut torrent_index = vec![];
            for (info_hash, _) in peers_lock.iter().skip(start) {
                torrent_index.push(*info_hash);
                if torrent_index.len() == size {
                    break;
                }
            }
            drop(peers_lock);

            let mut peers = vec![];
            let torrents = self.get_torrents(torrent_index.clone()).await;
            for (info_hash, torrent_entry) in torrents.iter() {
                if torrent_entry.is_some() {
                    let torrent = torrent_entry.clone().unwrap().clone();
                    for (peer_id, torrent_peer) in torrent.peers.iter() {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            peers.push((*info_hash, *peer_id));
                        }
                    }
                } else {
                    continue;
                }
            }
            removed_peers += peers.len() as u64;
            let _ = self.remove_peers(peers, self.config.clone().persistence).await;

            if torrent_index.len() != size {
                break;
            }

            start += size;
        }
        info!("[PEERS] Removed {} peers", removed_peers);
    }
}