use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::announce_entry::AnnounceEntry;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_peers::TorrentPeers;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::types::ahash_map::AHashMap;
use log::info;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;

impl TorrentTracker {
    /// Returns up to `amount` seeds and peers of a torrent, filtered by IP family and excluding
    /// `self_peer_id`. Returns `None` when the torrent is unknown.
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

    /// Copies up to `amount` peers out of a peer map, filtered by IP family and excluding
    /// `self_peer_id`. An `amount` of 0 means unlimited.
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

    /// Borrowing variant of [`TorrentTracker::get_peers`]: returns references instead of clones,
    /// for building responses without copying peer data. An `amount` of 0 means unlimited.
    #[inline]
    pub fn get_peers_ref<'a>(&self, peers: &'a AHashMap<PeerId, TorrentPeer>, type_ip: TorrentPeersType, self_peer_id: Option<PeerId>, amount: usize) -> Vec<(&'a PeerId, &'a TorrentPeer)>
    {
        let mut result = Vec::with_capacity(amount.min(peers.len()));
        for (peer_id, torrent_peer) in peers {
            if amount != 0 && result.len() >= amount {
                break;
            }
            let peer_addr = &torrent_peer.peer_addr;
            let ip_type_match = match type_ip {
                TorrentPeersType::All => peer_addr.is_ipv4() || peer_addr.is_ipv6(),
                TorrentPeersType::IPv4 => peer_addr.is_ipv4(),
                TorrentPeersType::IPv6 => peer_addr.is_ipv6(),
            };
            if ip_type_match && self_peer_id.is_none_or(|id| id != *peer_id) {
                result.push((peer_id, torrent_peer));
            }
        }
        result
    }

    /// Inserts or refreshes a peer in the torrent's swarm, creating the torrent when needed.
    ///
    /// The peer is classified as seed or leecher (`left == 0` -> seed), IPv4/IPv6 or RTC, and any
    /// previous classification of the same peer id is removed first so statistics stay exact.
    /// Pending RTC answers survive re-announces. Set `completed` to also count a finished download.
    ///
    /// Returns a bounded [`AnnounceEntry`] snapshot for building the response.
    pub fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool) -> AnnounceEntry
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
                if torrent_peer.is_rtctorrent() {
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
                let snapshot = AnnounceEntry::from_entry(&torrent_entry);
                v.insert(torrent_entry);
                snapshot
            }
            Entry::Occupied(mut o) => {
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
                    .and_then(|p| p.rtc_data.as_ref())
                    .map(|rtc| rtc.pending_answers.clone())
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
                if torrent_peer.is_rtctorrent() {
                    if torrent_peer.left == NumberOfBytes(0) {
                        self.update_stats(StatsEvent::Seeds, 1);
                        let mut new_peer = torrent_peer;
                        if !old_rtc_pending_answers.is_empty()
                            && let Some(ref mut rtc) = new_peer.rtc_data {
                            rtc.pending_answers = old_rtc_pending_answers;
                        }
                        entry.rtc_seeds.insert(peer_id, new_peer);
                    } else {
                        self.update_stats(StatsEvent::Peers, 1);
                        let mut new_peer = torrent_peer;
                        if !old_rtc_pending_answers.is_empty()
                            && let Some(ref mut rtc) = new_peer.rtc_data {
                            rtc.pending_answers = old_rtc_pending_answers;
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
                AnnounceEntry::from_entry(entry)
            }
        }
    }

    /// Removes a peer from a torrent. Returns `(existed, remaining)` where `existed` is true
    /// when the torrent entry was found, and `remaining` is a snapshot of the torrent after
    /// removal (or `None` when the torrent itself was removed because it became empty and is
    /// not persistent).
    pub fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool, cleanup: bool) -> (bool, Option<AnnounceEntry>)
    {
        if !self.torrents_sharding.contains_peer(info_hash, peer_id) {
            return (false, None);
        }
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(_) => (false, None),
            Entry::Occupied(mut o) => {
                if cleanup {
                    info!("[PEERS] Removing from torrent {info_hash} peer {peer_id}");
                }
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
                    (true, None)
                } else {
                    (true, Some(AnnounceEntry::from_entry(entry)))
                }
            }
        }
    }
}