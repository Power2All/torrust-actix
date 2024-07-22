use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use log::info;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_peers::TorrentPeers;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
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

    pub fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool) -> (Option<TorrentEntry>, TorrentEntry)
    {
        match self.get_torrent(info_hash) {
            None => {
                let mut torrent_entry = TorrentEntry {
                    seeds: BTreeMap::new(),
                    peers: BTreeMap::new(),
                    completed: if completed && torrent_peer.left == NumberOfBytes(0) { 1 } else { 0 },
                    updated: std::time::Instant::now()
                };
                match torrent_peer.left {
                    NumberOfBytes(0) => {
                        torrent_entry.seeds.insert(peer_id, torrent_peer);
                    }
                    _ => {
                        torrent_entry.peers.insert(peer_id, torrent_peer);
                    }
                }
                self.add_torrent(info_hash, torrent_entry.clone());
                (None, torrent_entry)
            }
            Some(mut torrent) => {
                let previous_torrent = torrent.clone();
                if completed {
                    torrent.completed += 1;
                }
                match torrent_peer.left {
                    NumberOfBytes(0) => {
                        torrent.seeds.insert(peer_id, torrent_peer);
                    }
                    _ => {
                        torrent.peers.insert(peer_id, torrent_peer);
                    }
                }
                (Some(previous_torrent), torrent)
            }
        }
    }

    pub fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool) -> (Option<TorrentEntry>, Option<TorrentEntry>)
    {
        match self.get_torrent(info_hash) {
            None => {
                (None, None)
            }
            Some(mut torrent) => {
                let previous_torrent = torrent.clone();
                torrent.seeds.remove(&peer_id);
                torrent.peers.remove(&peer_id);
                if !persistent && torrent.seeds.is_empty() && torrent.peers.is_empty() {
                    self.remove_torrent(info_hash);
                    return (Some(previous_torrent), None);
                }
                (Some(previous_torrent), Some(torrent))
            }
        }
    }

    pub fn torrent_peers_cleanup(&self, peer_timeout: Duration, persistent: bool) -> (u64, u64, u64)
    {
        let mut torrents_removed = 0u64;
        let mut seeds_found = 0u64;
        let mut peers_found = 0u64;
        for shard in 0u8..=255u8 {
            let shard = self.torrents_sharding.clone().get_shard(shard).unwrap();
            if !shard.is_empty() {
                for torrent_entry in shard.iter() {
                    for (peer_id, torrent_peer) in torrent_entry.value().seeds.iter() {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            match self.remove_torrent_peer(*torrent_entry.key(), *peer_id, persistent) {
                                (None, None) => {
                                    torrents_removed += 1;
                                }
                                (previous, None) => {
                                    torrents_removed += 1;
                                    seeds_found += previous.clone().unwrap().seeds.len() as u64;
                                    peers_found += previous.clone().unwrap().peers.len() as u64;
                                }
                                (previous, new) => {
                                    seeds_found += previous.clone().unwrap().seeds.len() as u64 - new.clone().unwrap().seeds.len() as u64;
                                    peers_found += previous.clone().unwrap().peers.len() as u64 - new.clone().unwrap().peers.len() as u64;
                                }
                            }
                        }
                    }
                    for (peer_id, torrent_peer) in torrent_entry.value().peers.iter() {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            match self.remove_torrent_peer(*torrent_entry.key(), *peer_id, persistent) {
                                (None, None) => {
                                    torrents_removed += 1;
                                }
                                (previous, None) => {
                                    torrents_removed += 1;
                                    seeds_found += previous.clone().unwrap().seeds.len() as u64;
                                    peers_found += previous.clone().unwrap().peers.len() as u64;
                                }
                                (previous, new) => {
                                    seeds_found += previous.clone().unwrap().seeds.len() as u64 - new.clone().unwrap().seeds.len() as u64;
                                    peers_found += previous.clone().unwrap().peers.len() as u64 - new.clone().unwrap().peers.len() as u64;
                                }
                            }
                        }
                    }
                }
            }
        }
        info!("[PEERS CLEANUP] Removed {} torrents, {} seeds and {} peers", torrents_removed, seeds_found, peers_found);
        (torrents_removed, seeds_found, peers_found)
    }
}
