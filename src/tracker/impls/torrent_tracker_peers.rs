use std::time::Duration;
use log::{debug, info};
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        debug!("[DEBUG] Calling get_torrent");
        let mut torrent = match self.get_torrent(info_hash).await {
            None => {
                debug!("[DEBUG] Calling add_torrent");
                self.add_torrent(info_hash, TorrentEntry::new(), persistent).await;
                TorrentEntry::new()
            }
            Some(torrent) => {
                torrent
            }
        };
        let seed = torrent.seeds.remove(&peer_id);
        let peer = torrent.peers.remove(&peer_id);
        if torrent_peer.left == NumberOfBytes(0) {
            if completed {
                torrent.completed += 1;
                self.update_stats(StatsEvent::Completed, 1).await;
                if persistent {
                    self.add_torrents_update(info_hash, torrent.completed).await;
                }
            }
            if seed.is_none() && peer.is_none() {
                torrent.seeds_count += 1;
            }
            torrent.seeds.insert(peer_id, torrent_peer);
        } else {
            if completed {
                torrent.completed += 1;
                self.update_stats(StatsEvent::Completed, 1).await;
                if persistent {
                    self.add_torrents_update(info_hash, torrent.completed).await;
                }
            }
            if seed.is_none() && peer.is_none() {
                torrent.peers_count += 1;
            }
            torrent.peers.insert(peer_id, torrent_peer);
        }
        debug!("[DEBUG] Calling add_torrent");
        self.add_torrent(info_hash, torrent.clone(), persistent).await;
        torrent
    }

    pub async fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool) -> Option<TorrentEntry>
    {
        debug!("[DEBUG] Calling get_torrent");
        let mut torrent = match self.get_torrent(info_hash).await {
            None => { return None; }
            Some(torrent) => { torrent }
        };
        let seed = torrent.seeds.remove(&peer_id);
        let peer = torrent.peers.remove(&peer_id);
        if seed.is_some() {
            torrent.seeds_count -= 1;
        }
        if peer.is_some() {
            torrent.peers_count -= 1;
        }
        if !persistent && torrent.seeds_count <= 0 && torrent.peers_count <= 0 {
            debug!("[DEBUG] Calling remove_torrent");
            self.remove_torrent(info_hash, persistent).await;
            return Some(torrent);
        }
        debug!("[DEBUG] Calling add_torrent");
        self.add_torrent(info_hash, torrent.clone(), persistent).await;
        Some(torrent)
    }

    pub async fn remove_torrent_peers(&self, peers: Vec<(InfoHash, PeerId)>, persistent: bool) -> Vec<(InfoHash, PeerId)>
    {
        let mut return_torrententries = Vec::new();
        for (info_hash, peer_id) in peers.iter() {
            debug!("[DEBUG] Calling remove_torrent_peer");
            self.remove_torrent_peer(*info_hash, *peer_id, persistent).await;
            return_torrententries.push((*info_hash, *peer_id));
        }
        return_torrententries
    }

    pub async fn torrent_peers_cleanup(&self, peer_timeout: Duration, persistent: bool)
    {
        let torrents_arc = self.torrents.clone();
        let mut start: usize = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        let mut removed_peers = 0u64;
        loop {
            if start > torrents_arc.len() {
                break;
            }
            info!("[PEERS CLEANUP] Scanning torrents from {} to  {}", start, (start + size));
            let mut torrent_index = vec![];
            for torrent in torrents_arc.iter().skip(start) {
                torrent_index.push(*torrent.key());
                if torrent_index.len() == size {
                    break;
                }
            }
            let mut remove_peers = vec![];
            for (info_hash, torrent_entry) in self.get_torrents(torrent_index).await {
                match torrent_entry {
                    None => {}
                    Some(torrent) => {
                        for (peer_id, torrent_peer) in torrent.seeds.iter() {
                            if torrent_peer.updated.elapsed() > peer_timeout {
                                remove_peers.push((info_hash, *peer_id));
                            }
                        }
                        for (peer_id, torrent_peer) in torrent.peers.iter() {
                            if torrent_peer.updated.elapsed() > peer_timeout {
                                remove_peers.push((info_hash, *peer_id));
                            }
                        }
                    }
                }
            }
            if !remove_peers.is_empty() {
                removed_peers += self.remove_torrent_peers(remove_peers, persistent).await.len() as u64;
            }
            start += size - 100;
        }
        info!("[PEERS CLEANUP] Removed {} peers", removed_peers);
    }
}
