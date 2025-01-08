use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use futures_util::future::join_all;
use log::info;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_peers::TorrentPeers;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub fn get_torrent_peers(&self, info_hash: InfoHash, amount: usize, ip_type: TorrentPeersType, self_ip: Option<IpAddr>) -> Option<TorrentPeers>
    {
        let mut returned_data = TorrentPeers {
            seeds_ipv4: BTreeMap::new(),
            seeds_ipv6: BTreeMap::new(),
            peers_ipv4: BTreeMap::new(),
            peers_ipv6: BTreeMap::new()
        };
        match self.get_torrent(info_hash) {
            None => { None }
            Some(data) => {
                match ip_type {
                    TorrentPeersType::All => {
                        returned_data.seeds_ipv4 = self.get_peers(data.seeds.clone(), TorrentPeersType::IPv4, self_ip, amount).unwrap_or_default();
                        returned_data.seeds_ipv6 = self.get_peers(data.seeds.clone(), TorrentPeersType::IPv6, self_ip, amount).unwrap_or_default();
                        returned_data.peers_ipv4 = self.get_peers(data.peers.clone(), TorrentPeersType::IPv4, self_ip, amount).unwrap_or_default();
                        returned_data.peers_ipv6 = self.get_peers(data.peers.clone(), TorrentPeersType::IPv6, self_ip, amount).unwrap_or_default();
                    }
                    TorrentPeersType::IPv4 => {
                        returned_data.seeds_ipv4 = self.get_peers(data.seeds.clone(), TorrentPeersType::IPv4, self_ip, amount).unwrap_or_default();
                        returned_data.peers_ipv4 = self.get_peers(data.peers.clone(), TorrentPeersType::IPv4, self_ip, amount).unwrap_or_default();
                    }
                    TorrentPeersType::IPv6 => {
                        returned_data.seeds_ipv6 = self.get_peers(data.seeds.clone(), TorrentPeersType::IPv6, self_ip, amount).unwrap_or_default();
                        returned_data.peers_ipv6 = self.get_peers(data.peers.clone(), TorrentPeersType::IPv6, self_ip, amount).unwrap_or_default();
                    }
                }
                Some(returned_data)
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_peers(&self, peers: BTreeMap<PeerId, TorrentPeer>, type_ip: TorrentPeersType, self_ip: Option<IpAddr>, amount: usize) -> Option<BTreeMap<PeerId, TorrentPeer>>
    {
        if amount != 0 {
            return peers.iter().take(amount).map(|(peer_id, torrent_peer)| {
                match type_ip {
                    TorrentPeersType::All => { None }
                    TorrentPeersType::IPv4 => {
                        match self_ip {
                            None => {
                                match torrent_peer.peer_addr {
                                    SocketAddr::V4(_) => { Some((*peer_id, torrent_peer.clone())) }
                                    SocketAddr::V6(_) => { None}
                                }
                            }
                            Some(ip) => {
                                if ip != torrent_peer.peer_addr.ip() {
                                    match torrent_peer.peer_addr {
                                        SocketAddr::V4(_) => { Some((*peer_id, torrent_peer.clone())) }
                                        SocketAddr::V6(_) => { None }
                                    }
                                } else {
                                    None
                                }
                            }
                        }
                    }
                    TorrentPeersType::IPv6 => {
                        match self_ip {
                            None => {
                                match torrent_peer.peer_addr {
                                    SocketAddr::V4(_) => { None }
                                    SocketAddr::V6(_) => { Some((*peer_id, torrent_peer.clone())) }
                                }
                            }
                            Some(ip) => {
                                if ip != torrent_peer.peer_addr.ip() {
                                    match torrent_peer.peer_addr {
                                        SocketAddr::V4(_) => { None }
                                        SocketAddr::V6(_) => { Some((*peer_id, torrent_peer.clone())) }
                                    }
                                } else {
                                    None
                                }
                            }
                        }
                    }
                }
            }).collect();
        }
        peers.iter().map(|(peer_id, torrent_peer)| {
            match type_ip {
                TorrentPeersType::All => { None }
                TorrentPeersType::IPv4 => {
                    match self_ip {
                        None => {
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => { Some((*peer_id, torrent_peer.clone())) }
                                SocketAddr::V6(_) => { None}
                            }
                        }
                        Some(ip) => {
                            if ip != torrent_peer.peer_addr.ip() {
                                match torrent_peer.peer_addr {
                                    SocketAddr::V4(_) => { Some((*peer_id, torrent_peer.clone())) }
                                    SocketAddr::V6(_) => { None }
                                }
                            } else {
                                None
                            }
                        }
                    }
                }
                TorrentPeersType::IPv6 => {
                    match self_ip {
                        None => {
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => { None }
                                SocketAddr::V6(_) => { Some((*peer_id, torrent_peer.clone())) }
                            }
                        }
                        Some(ip) => {
                            if ip != torrent_peer.peer_addr.ip() {
                                match torrent_peer.peer_addr {
                                    SocketAddr::V4(_) => { None }
                                    SocketAddr::V6(_) => { Some((*peer_id, torrent_peer.clone())) }
                                }
                            } else {
                                None
                            }
                        }
                    }
                }
            }
        }).collect()
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool) -> (Option<TorrentEntry>, TorrentEntry)
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                let mut torrent_entry = TorrentEntry {
                    seeds: BTreeMap::new(),
                    peers: BTreeMap::new(),
                    completed: if completed && torrent_peer.left == NumberOfBytes(0) { self.update_stats(StatsEvent::Completed, 1); 1 } else { 0 },
                    updated: std::time::Instant::now()
                };
                self.update_stats(StatsEvent::Torrents, 1);
                match torrent_peer.left {
                    NumberOfBytes(0) => {
                        self.update_stats(StatsEvent::Seeds, 1);
                        torrent_entry.seeds.insert(peer_id, torrent_peer);
                    }
                    _ => {
                        self.update_stats(StatsEvent::Peers, 1);
                        torrent_entry.peers.insert(peer_id, torrent_peer);
                    }
                }
                v.insert(torrent_entry.clone());
                (None, torrent_entry)
            }
            Entry::Occupied(mut o) => {
                let previous_torrent = o.get().clone();
                if o.get_mut().seeds.remove(&peer_id).is_some() {
                    self.update_stats(StatsEvent::Seeds, -1);
                };
                if o.get_mut().peers.remove(&peer_id).is_some() {
                    self.update_stats(StatsEvent::Peers, -1);
                };
                if completed {
                    self.update_stats(StatsEvent::Completed, 1);
                    o.get_mut().completed += 1;
                }
                match torrent_peer.left {
                    NumberOfBytes(0) => {
                        self.update_stats(StatsEvent::Seeds, 1);
                        o.get_mut().seeds.insert(peer_id, torrent_peer);
                    }
                    _ => {
                        self.update_stats(StatsEvent::Peers, 1);
                        o.get_mut().peers.insert(peer_id, torrent_peer);
                    }
                }
                (Some(previous_torrent), o.get().clone())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_torrent_peers(&self, shard: u8, peers: Vec<(InfoHash, PeerId)>, persistent: bool) -> Vec<(InfoHash, Option<TorrentEntry>, Option<TorrentEntry>)>
    {
        let mut return_data = vec![];
        let shard = self.torrents_sharding.clone().get_shard(shard).unwrap();
        let mut lock = shard.write();
        for (info_hash, peer_id) in peers {
            match lock.entry(info_hash) {
                Entry::Vacant(_) => {
                    return_data.push((info_hash, None, None));
                }
                Entry::Occupied(mut o) => {
                    let previous_torrent = o.get().clone();
                    if o.get_mut().seeds.remove(&peer_id).is_some() {
                        self.update_stats(StatsEvent::Seeds, -1);
                    };
                    if o.get_mut().peers.remove(&peer_id).is_some() {
                        self.update_stats(StatsEvent::Peers, -1);
                    };
                    if !persistent && o.get().seeds.is_empty() && o.get().peers.is_empty() {
                        lock.remove(&info_hash);
                        self.update_stats(StatsEvent::Torrents, -1);
                        return_data.push((info_hash, Some(previous_torrent), None));
                    } else {
                        return_data.push((info_hash, Some(previous_torrent), Some(o.get().clone())));
                    }
                }
            }
        }
        return_data
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool) -> (Option<TorrentEntry>, Option<TorrentEntry>)
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(_) => {
                (None, None)
            }
            Entry::Occupied(mut o) => {
                let previous_torrent = o.get().clone();
                if o.get_mut().seeds.remove(&peer_id).is_some() {
                    self.update_stats(StatsEvent::Seeds, -1);
                };
                if o.get_mut().peers.remove(&peer_id).is_some() {
                    self.update_stats(StatsEvent::Peers, -1);
                };
                if !persistent && o.get().seeds.is_empty() && o.get().peers.is_empty() {
                    lock.remove(&info_hash);
                    self.update_stats(StatsEvent::Torrents, -1);
                    return (Some(previous_torrent), None);
                }
                (Some(previous_torrent), Some(o.get().clone()))
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn torrent_peers_cleanup(&self, torrent_tracker: Arc<TorrentTracker>, peer_timeout: Duration, persistent: bool)
    {
        let torrents_removed = Arc::new(AtomicU64::new(0));
        let seeds_found = Arc::new(AtomicU64::new(0));
        let peers_found = Arc::new(AtomicU64::new(0));
        let mut threads = vec![];
        for shard in 0u8..=255u8 {
            let torrent_tracker_clone = torrent_tracker.clone();
            let shard_data = torrent_tracker.torrents_sharding.get_shard_content(shard);
            if !shard_data.is_empty() {
                let torrents_removed_clone = torrents_removed.clone();
                let seeds_found_clone = seeds_found.clone();
                let peers_found_clone = peers_found.clone();
                threads.push(tokio::spawn(async move {
                    let mut seeds = 0u64;
                    let mut peers = 0u64;
                    let mut remove_list = vec![];
                    for (info_hash, torrent_entry) in shard_data.iter() {
                        for (peer_id, torrent_peer) in torrent_entry.seeds.iter() {
                            seeds += 1;
                            if torrent_peer.updated.elapsed() > peer_timeout {
                                remove_list.push((*info_hash, *peer_id));
                            }
                        }
                        for (peer_id, torrent_peer) in torrent_entry.peers.iter() {
                            peers += 1;
                            if torrent_peer.updated.elapsed() > peer_timeout {
                                remove_list.push((*info_hash, *peer_id));
                            }
                        }
                    }
                    for (_, previous, next) in torrent_tracker_clone.remove_torrent_peers(shard, remove_list, persistent).iter() {
                        match (previous, next) {
                            (None, None) => {
                                torrents_removed_clone.fetch_add(1, Ordering::SeqCst);
                            }
                            (previous, None) => {
                                torrents_removed_clone.fetch_add(1, Ordering::SeqCst);
                                seeds_found_clone.fetch_add(previous.clone().unwrap().seeds.len() as u64, Ordering::SeqCst);
                                peers_found_clone.fetch_add(previous.clone().unwrap().peers.len() as u64, Ordering::SeqCst);
                            }
                            (previous, new) => {
                                seeds_found_clone.fetch_add(previous.clone().unwrap().seeds.len() as u64 - new.clone().unwrap().seeds.len() as u64, Ordering::SeqCst);
                                peers_found_clone.fetch_add(previous.clone().unwrap().peers.len() as u64 - new.clone().unwrap().peers.len() as u64, Ordering::SeqCst);
                            }
                        }
                    }
                    info!("[PEERS CLEANUP] Scanned {} seeds and {} peers", seeds, peers);
                }));
            }
        }
        join_all(threads).await;

        info!("[PEERS CLEANUP] Removed {} torrents, {} seeds and {} peers", torrents_removed.clone().load(Ordering::SeqCst), seeds_found.clone().load(Ordering::SeqCst), peers_found.clone().load(Ordering::SeqCst));
    }
}