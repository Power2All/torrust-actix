use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_peers::TorrentPeers;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::types::ahash_map::AHashMap;
use log::info;
use std::collections::btree_map::Entry;
use std::net::SocketAddr;

impl TorrentTracker {
    pub fn get_torrent_peers(&self, info_hash: InfoHash, amount: usize, ip_type: TorrentPeersType, self_peer_id: Option<PeerId>) -> Option<TorrentPeers>
    {
        self.get_torrent(info_hash).map(|data| {
            let mut returned_data = TorrentPeers {
                seeds_ipv4: AHashMap::default(),
                seeds_ipv6: AHashMap::default(),
                peers_ipv4: AHashMap::default(),
                peers_ipv6: AHashMap::default()
            };
            match ip_type {
                TorrentPeersType::All => {
                    returned_data.seeds_ipv4 = self.get_peers(&data.seeds, TorrentPeersType::IPv4, self_peer_id, amount);
                    returned_data.seeds_ipv6 = self.get_peers(&data.seeds_ipv6, TorrentPeersType::IPv6, self_peer_id, amount);
                    returned_data.peers_ipv4 = self.get_peers(&data.peers, TorrentPeersType::IPv4, self_peer_id, amount);
                    returned_data.peers_ipv6 = self.get_peers(&data.peers_ipv6, TorrentPeersType::IPv6, self_peer_id, amount);
                }
                TorrentPeersType::IPv4 => {
                    returned_data.seeds_ipv4 = self.get_peers(&data.seeds, TorrentPeersType::IPv4, self_peer_id, amount);
                    returned_data.peers_ipv4 = self.get_peers(&data.peers, TorrentPeersType::IPv4, self_peer_id, amount);
                }
                TorrentPeersType::IPv6 => {
                    returned_data.seeds_ipv6 = self.get_peers(&data.seeds_ipv6, TorrentPeersType::IPv6, self_peer_id, amount);
                    returned_data.peers_ipv6 = self.get_peers(&data.peers_ipv6, TorrentPeersType::IPv6, self_peer_id, amount);
                }
            }
            returned_data
        })
    }

    #[inline]
    pub fn get_peers(&self, peers: &AHashMap<PeerId, TorrentPeer>, type_ip: TorrentPeersType, self_peer_id: Option<PeerId>, amount: usize) -> AHashMap<PeerId, TorrentPeer>
    {
        let should_include = |peer_id: &PeerId, peer_addr: &SocketAddr| -> bool {
            let ip_type_match = match type_ip {
                TorrentPeersType::All => peer_addr.is_ipv4() || peer_addr.is_ipv6(),
                TorrentPeersType::IPv4 => peer_addr.is_ipv4(),
                TorrentPeersType::IPv6 => peer_addr.is_ipv6(),
            };
            ip_type_match && self_peer_id.is_none_or(|id| id != *peer_id)
        };
        let mut result = AHashMap::default();
        result.reserve(amount.min(peers.len()));
        for (peer_id, torrent_peer) in peers {
            if amount != 0 && result.len() >= amount {
                break;
            }
            if should_include(peer_id, &torrent_peer.peer_addr) {
                result.insert(*peer_id, torrent_peer.clone());
            }
        }
        result
    }

    pub fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool) -> (Option<TorrentEntry>, TorrentEntry)
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                let mut torrent_entry = TorrentEntry {
                    seeds: AHashMap::default(),
                    seeds_ipv6: AHashMap::default(),
                    peers: AHashMap::default(),
                    peers_ipv6: AHashMap::default(),
                    rtc_seeds: AHashMap::default(),
                    rtc_peers: AHashMap::default(),
                    completed: 0,
                    updated: std::time::Instant::now()
                };
                if completed && torrent_peer.left == NumberOfBytes(0) {
                    self.update_stats(StatsEvent::Completed, 1);
                    torrent_entry.completed = 1;
                }
                self.update_stats(StatsEvent::Torrents, 1);
                if torrent_peer.is_rtctorrent {
                    if torrent_peer.left == NumberOfBytes(0) {
                        self.update_stats(StatsEvent::Seeds, 1);
                        torrent_entry.rtc_seeds.insert(peer_id, torrent_peer);
                    } else {
                        self.update_stats(StatsEvent::Peers, 1);
                        torrent_entry.rtc_peers.insert(peer_id, torrent_peer);
                    }
                } else if torrent_peer.left == NumberOfBytes(0) {
                    self.update_stats(StatsEvent::Seeds, 1);
                    if torrent_peer.peer_addr.is_ipv4() {
                        torrent_entry.seeds.insert(peer_id, torrent_peer);
                    } else {
                        torrent_entry.seeds_ipv6.insert(peer_id, torrent_peer);
                    }
                } else {
                    self.update_stats(StatsEvent::Peers, 1);
                    if torrent_peer.peer_addr.is_ipv4() {
                        torrent_entry.peers.insert(peer_id, torrent_peer);
                    } else {
                        torrent_entry.peers_ipv6.insert(peer_id, torrent_peer);
                    }
                }
                let entry_clone = torrent_entry.clone();
                v.insert(torrent_entry);
                (None, entry_clone)
            }
            Entry::Occupied(mut o) => {
                let previous_torrent = o.get().clone();
                let entry = o.get_mut();
                let (seeds_removed, peers_removed) = if torrent_peer.peer_addr.is_ipv4() {
                    (
                        i64::from(entry.seeds.remove(&peer_id).is_some()),
                        i64::from(entry.peers.remove(&peer_id).is_some()),
                    )
                } else {
                    (
                        i64::from(entry.seeds_ipv6.remove(&peer_id).is_some()),
                        i64::from(entry.peers_ipv6.remove(&peer_id).is_some()),
                    )
                };
                let old_rtc_pending_answers = entry.rtc_seeds.get(&peer_id)
                    .or_else(|| entry.rtc_peers.get(&peer_id))
                    .map(|p| p.rtc_pending_answers.clone())
                    .unwrap_or_default();
                let was_rtc_seed = entry.rtc_seeds.remove(&peer_id).is_some();
                let was_rtc_peer = entry.rtc_peers.remove(&peer_id).is_some();
                if seeds_removed > 0 {
                    self.update_stats(StatsEvent::Seeds, -seeds_removed);
                }
                if peers_removed > 0 {
                    self.update_stats(StatsEvent::Peers, -peers_removed);
                }
                if was_rtc_seed {
                    self.update_stats(StatsEvent::Seeds, -1);
                }
                if was_rtc_peer {
                    self.update_stats(StatsEvent::Peers, -1);
                }
                if completed {
                    self.update_stats(StatsEvent::Completed, 1);
                    entry.completed += 1;
                }

                if torrent_peer.is_rtctorrent {
                    if torrent_peer.left == NumberOfBytes(0) {
                        self.update_stats(StatsEvent::Seeds, 1);
                        let mut new_peer = torrent_peer;
                        if !old_rtc_pending_answers.is_empty() {
                            new_peer.rtc_pending_answers = old_rtc_pending_answers;
                        }
                        entry.rtc_seeds.insert(peer_id, new_peer);
                    } else {
                        self.update_stats(StatsEvent::Peers, 1);
                        let mut new_peer = torrent_peer;
                        if !old_rtc_pending_answers.is_empty() {
                            new_peer.rtc_pending_answers = old_rtc_pending_answers;
                        }
                        entry.rtc_peers.insert(peer_id, new_peer);
                    }
                } else if torrent_peer.left == NumberOfBytes(0) {
                    self.update_stats(StatsEvent::Seeds, 1);
                    if torrent_peer.peer_addr.is_ipv4() {
                        entry.seeds.insert(peer_id, torrent_peer);
                    } else {
                        entry.seeds_ipv6.insert(peer_id, torrent_peer);
                    }
                } else {
                    self.update_stats(StatsEvent::Peers, 1);
                    if torrent_peer.peer_addr.is_ipv4() {
                        entry.peers.insert(peer_id, torrent_peer);
                    } else {
                        entry.peers_ipv6.insert(peer_id, torrent_peer);
                    }
                }
                entry.updated = std::time::Instant::now();
                (Some(previous_torrent), entry.clone())
            }
        }
    }

    pub fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool, cleanup: bool) -> (Option<TorrentEntry>, Option<TorrentEntry>)
    {
        if !self.torrents_sharding.contains_peer(info_hash, peer_id) {
            return (None, None);
        }
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(_) => (None, None),
            Entry::Occupied(mut o) => {
                if cleanup {
                    info!("[PEERS] Removing from torrent {info_hash} peer {peer_id}");
                }
                let previous_torrent = o.get().clone();
                let entry = o.get_mut();
                let seeds_removed = i64::from(entry.seeds.remove(&peer_id).is_some())
                    + i64::from(entry.seeds_ipv6.remove(&peer_id).is_some());
                let peers_removed = i64::from(entry.peers.remove(&peer_id).is_some())
                    + i64::from(entry.peers_ipv6.remove(&peer_id).is_some());
                let was_rtc_seed = entry.rtc_seeds.remove(&peer_id).is_some();
                let was_rtc_peer = entry.rtc_peers.remove(&peer_id).is_some();
                if seeds_removed > 0 {
                    self.update_stats(StatsEvent::Seeds, -seeds_removed);
                }
                if peers_removed > 0 {
                    self.update_stats(StatsEvent::Peers, -peers_removed);
                }
                if was_rtc_seed {
                    self.update_stats(StatsEvent::Seeds, -1);
                }
                if was_rtc_peer {
                    self.update_stats(StatsEvent::Peers, -1);
                }
                if !persistent && entry.seeds.is_empty() && entry.seeds_ipv6.is_empty() && entry.peers.is_empty() && entry.peers_ipv6.is_empty() && entry.rtc_seeds.is_empty() && entry.rtc_peers.is_empty() {
                    o.remove();
                    self.update_stats(StatsEvent::Torrents, -1);
                    (Some(previous_torrent), None)
                } else {
                    (Some(previous_torrent), Some(entry.clone()))
                }
            }
        }
    }
}