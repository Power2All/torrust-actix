use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::net::{IpAddr, SocketAddr};
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
    pub fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool, cleanup: bool) -> (Option<TorrentEntry>, Option<TorrentEntry>)
    {
        if !self.torrents_sharding.contains_peer(info_hash, peer_id) { return (None, None); }
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(_) => {
                (None, None)
            }
            Entry::Occupied(mut o) => {
                if cleanup {
                    info!("[PEERS] Removing from torrent {info_hash} peer {peer_id}");
                }
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
}