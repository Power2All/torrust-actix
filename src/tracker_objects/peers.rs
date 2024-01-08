use log::info;
use std::collections::BTreeMap;
use std::time::Duration;

use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;
use crate::tracker_objects::torrents::TorrentEntry;

impl TorrentTracker {
    pub async fn add_peer(&self, info_hash: InfoHash, peer_id: PeerId, peer_entry: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let mut added_seeder = false;
        let mut added_leecher = false;
        let mut removed_seeder = false;
        let mut removed_leecher = false;
        let mut completed_applied = false;

        let torrents_arc = self.torrents.clone();
        let peers_arc = self.peers.clone();

        let torrent = match torrents_arc.get(&info_hash) {
            None => { TorrentEntry::new() }
            Some(data) => {
                let mut peers = match peers_arc.get(&info_hash) {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers.value().clone() }
                };
                let mut data_torrent = data.value().clone();

                match peers.get(&peer_id) {
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

                torrents_arc.insert(info_hash, data_torrent.clone());
                peers_arc.insert(info_hash, peers.clone());

                if !persistent {
                    TorrentEntry {
                        peers,
                        completed: 0,
                        seeders: data_torrent.seeders,
                        leechers: data_torrent.leechers,
                    }
                } else {
                    TorrentEntry {
                        peers,
                        completed: data_torrent.completed,
                        seeders: data_torrent.seeders,
                        leechers: data_torrent.leechers,
                    }
                }
            }
        };

        if persistent && completed { self.add_torrents_update(info_hash, torrent.completed).await; }
        if added_seeder { self.update_stats(StatsEvent::Seeds, 1).await; }
        if added_leecher { self.update_stats(StatsEvent::Peers, 1).await; }
        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
        if completed_applied { self.update_stats(StatsEvent::Completed, 1).await; }

        torrent
    }

    pub async fn remove_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool) -> TorrentEntry
    {
        let mut removed_seeder = false;
        let mut removed_leecher = false;

        let torrents_arc = self.torrents.clone();
        let peers_arc = self.peers.clone();

        let torrent = match torrents_arc.get(&info_hash) {
            None => { TorrentEntry::new() }
            Some(data) => {
                let mut peers = match peers_arc.get(&info_hash) {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers.value().clone() }
                };
                let mut data_torrent = data.value().clone();
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

                torrents_arc.insert(info_hash, data_torrent.clone());
                if peers.is_empty() { peers_arc.remove(&info_hash); } else { peers_arc.insert(info_hash, peers.clone()); }
                if !persistent { torrents_arc.remove(&info_hash); }

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

    pub async fn remove_peers(&self, peers: Vec<(InfoHash, PeerId)>, persistent: bool) -> Vec<(InfoHash, PeerId)>
    {
        let mut removed_seeder = 0i64;
        let mut removed_leecher = 0i64;
        let mut return_torrententries = Vec::new();

        let torrents_arc = self.torrents.clone();
        let peers_arc = self.peers.clone();

        for (info_hash, peer_id) in peers.iter() {
            if let Some(data) = torrents_arc.get(info_hash) {
                let mut data_torrent = data.value().clone();
                let mut peers = match peers_arc.get(info_hash) {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers.value().clone() }
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

                torrents_arc.insert(*info_hash, data_torrent.clone());
                if peers.is_empty() { peers_arc.remove(info_hash); } else { peers_arc.insert(*info_hash, peers.clone()); }
                if !persistent { torrents_arc.remove(info_hash); }
            };
        }

        if removed_seeder != 0 { self.update_stats(StatsEvent::Seeds, removed_seeder).await; }
        if removed_leecher != 0 { self.update_stats(StatsEvent::Peers, removed_leecher).await; }

        return_torrententries
    }

    pub async fn clean_peers(&self, peer_timeout: Duration)
    {
        // Cleaning up peers in chunks, to prevent slow behavior.
        let peers_arc = self.peers.clone();

        let mut start: usize = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        let mut removed_peers = 0u64;

        loop {
            info!("[PEERS] Scanning peers {} to {}", start, (start + size));

            let mut torrent_index = vec![];
            for item in peers_arc.iter().skip(start) {
                torrent_index.push(*item.key());
                if torrent_index.len() == size { break; }
            }

            let mut peers = vec![];
            let torrents = self.get_torrents(torrent_index.clone()).await;
            for (info_hash, torrent_entry) in torrents.iter() {
                if torrent_entry.is_some() {
                    let torrent = torrent_entry.clone().unwrap().clone();
                    for (peer_id, torrent_peer) in torrent.peers.iter() {
                        if torrent_peer.updated.elapsed() > peer_timeout { peers.push((*info_hash, *peer_id)); }
                    }
                } else { continue; }
            }
            let response = self.remove_peers(peers.clone(), self.config.clone().persistence).await;
            removed_peers += response.len() as u64;

            if torrent_index.len() != size {
                break;
            }

            start += size;
        }
        info!("[PEERS] Removed {} peers", removed_peers);
    }
}