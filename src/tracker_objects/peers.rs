use async_std::future::timeout;
use log::{error, info};
use std::collections::BTreeMap;
use std::time::Duration;

use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;
use crate::tracker_objects::torrents::TorrentEntry;

impl TorrentTracker {
    pub async fn add_peer(&self, info_hash: InfoHash, peer_id: PeerId, peer_entry: TorrentPeer, completed: bool, persistent: bool) -> Result<TorrentEntry, ()>
    {
        let mut added_seeder = false;
        let mut added_leecher = false;
        let mut removed_seeder = false;
        let mut removed_leecher = false;
        let mut completed_applied = false;

        let torrent_input = match timeout(Duration::from_secs(30), async move {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.lock().await;
            let torrent_input = torrents_lock.get(&info_hash).cloned();
            drop(torrents_lock);
            torrent_input
        }).await {
            Ok(data) => { data }
            Err(_) => {
                error!("[ADD_PEER] Read Lock (torrents) request timed out!");
                return Err(());
            }
        };

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peer = match timeout(Duration::from_secs(30), async move {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.lock().await;
                    let peer = peers_lock.get(&info_hash).cloned();
                    drop(peers_lock);
                    peer
                }).await {
                    Ok(data) => { data }
                    Err(_) => {
                        error!("[ADD_PEER] Read Lock (peers) request timed out!");
                        return Err(());
                    }
                };

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
                let mut torrents_lock = torrents_arc.lock().await;
                torrents_lock.insert(info_hash, data_torrent.clone());
                drop(torrents_lock);

                let peers_arc = self.map_peers.clone();
                let mut peers_lock = peers_arc.lock().await;
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

        Ok(torrent)
    }

    pub async fn remove_peer(&self, info_hash: InfoHash, peer_id: PeerId, _persistent: bool) -> Result<TorrentEntry, ()>
    {
        let mut removed_seeder = false;
        let mut removed_leecher = false;

        let torrent_input = match timeout(Duration::from_secs(30), async move {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.lock().await;
            let torrent_input = torrents_lock.get(&info_hash).cloned();
            drop(torrents_lock);
            torrent_input
        }).await {
            Ok(data) => { data }
            Err(_) => {
                error!("[REMOVE_PEER] Read Lock (torrents) request timed out!");
                return Err(());
            }
        };

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peer = match timeout(Duration::from_secs(30), async move {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.lock().await;
                    let peer = peers_lock.get(&info_hash).cloned();
                    drop(peers_lock);
                    peer
                }).await {
                    Ok(data) => { data }
                    Err(_) => {
                        error!("[REMOVE_PEER] Read Lock (peers) request timed out!");
                        return Err(());
                    }
                };

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
                let mut torrents_lock = torrents_arc.lock().await;
                torrents_lock.insert(info_hash, data_torrent.clone());
                drop(torrents_lock);

                if peers.is_empty() {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.lock().await;
                    peers_lock.remove(&info_hash);
                    drop(peers_lock);
                } else {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.lock().await;
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

        Ok(torrent)
    }

    pub async fn remove_peers(&self, peers: Vec<(InfoHash, PeerId)>, _persistent: bool) -> Result<Vec<(InfoHash, PeerId)>, ()>
    {
        let mut removed_seeder = 0i64;
        let mut removed_leecher = 0i64;
        let mut return_torrententries = Vec::new();

        for (info_hash, peer_id) in peers.iter() {
            let torrent = match timeout(Duration::from_secs(30), async move {
                let torrents_arc = self.map_torrents.clone();
                let torrents_lock = torrents_arc.lock().await;
                let torrent = torrents_lock.get(info_hash).cloned();
                drop(torrents_lock);
                torrent
            }).await {
                Ok(data) => { data }
                Err(_) => {
                    error!("[REMOVE_PEERS] Read Lock (torrents) request timed out!");
                    return Err(());
                }
            };

            if let Some(mut data_torrent) = torrent {
                let peer = match timeout(Duration::from_secs(30), async move {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.lock().await;
                    let peer = peers_lock.get(info_hash).cloned();
                    drop(peers_lock);
                    peer
                }).await {
                    Ok(data) => { data }
                    Err(_) => {
                        error!("[REMOVE_PEERS] Read Lock (peers) request timed out!");
                        return Err(());
                    }
                };

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
                    return_torrententries.push((*info_hash, *peer_id));
                }

                let torrents_arc = self.map_torrents.clone();
                let mut torrents_lock = torrents_arc.lock().await;
                torrents_lock.insert(*info_hash, data_torrent.clone());
                drop(torrents_lock);

                if peers.is_empty() {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.lock().await;
                    peers_lock.remove(info_hash);
                    drop(peers_lock);
                } else {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.lock().await;
                    peers_lock.insert(*info_hash, peers.clone());
                    drop(peers_lock);
                }
            };
        }

        if removed_seeder != 0 { self.update_stats(StatsEvent::Seeds, removed_seeder).await; }
        if removed_leecher != 0 { self.update_stats(StatsEvent::Peers, removed_leecher).await; }

        Ok(return_torrententries)
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
            let peers_lock = peers_arc.lock().await;
            let mut torrent_index = vec![];
            for (info_hash, _) in peers_lock.iter().skip(start) {
                torrent_index.push(*info_hash);
                if torrent_index.len() == size {
                    break;
                }
            }
            drop(peers_lock);

            let mut peers = vec![];
            let torrents = match self.get_torrents(torrent_index.clone()).await {
                Ok(data_request) => { data_request }
                Err(_) => { continue; }
            };
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
            if let Ok(data_request) = self.remove_peers(peers.clone(), self.config.clone().persistence).await {
                removed_peers += data_request.len() as u64;
            } else {
                continue;
            }

            if torrent_index.len() != size {
                break;
            }

            start += size;
        }
        info!("[PEERS] Removed {} peers", removed_peers);
    }
}