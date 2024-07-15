use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
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
    pub async fn get_torrent_peers(&self, info_hash: InfoHash, amount: u64, ip_type: TorrentPeersType, self_ip: Option<IpAddr>) -> TorrentPeers
    {
        let mut torrent_peers = TorrentPeers {
            seeds_ipv4: BTreeMap::new(),
            seeds_ipv6: BTreeMap::new(),
            peers_ipv4: BTreeMap::new(),
            peers_ipv6: BTreeMap::new()
        };
        let map = self.torrents_map.clone();
        let lock = map.read();
        match lock.get(&info_hash) {
            None => {
                torrent_peers
            }
            Some(t) => {
                match ip_type {
                    TorrentPeersType::All => {
                        let mut count_seeds_ipv4 = 0u64;
                        let mut count_seeds_ipv6 = 0u64;
                        let mut count_peers_ipv4 = 0u64;
                        let mut count_peers_ipv6 = 0u64;
                        for (peer_id, torrent_peer) in t.seeds.iter() {
                            if amount != 0 && count_seeds_ipv4 == amount && count_seeds_ipv6 == amount {
                                break;
                            }
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => {
                                    if (amount != 0 && count_seeds_ipv4 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.seeds_ipv4.insert(*peer_id, torrent_peer.clone());
                                    count_seeds_ipv4 += 1;
                                }
                                SocketAddr::V6(_) => {
                                    if (amount != 0 && count_seeds_ipv6 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.seeds_ipv6.insert(*peer_id, torrent_peer.clone());
                                    count_seeds_ipv6 += 1;
                                }
                            }
                        }
                        for (peer_id, torrent_peer) in t.peers.iter() {
                            if count_peers_ipv4 == amount && count_peers_ipv6 == amount {
                                break;
                            }
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => {
                                    if (amount != 0 && count_peers_ipv4 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.peers_ipv4.insert(*peer_id, torrent_peer.clone());
                                    count_peers_ipv4 += 1;
                                }
                                SocketAddr::V6(_) => {
                                    if (amount != 0 && count_peers_ipv6 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.peers_ipv6.insert(*peer_id, torrent_peer.clone());
                                    count_peers_ipv6 += 1;
                                }
                            }
                        }
                    }
                    TorrentPeersType::IPv4 => {
                        let mut count_seeds_ipv4 = 0u64;
                        let mut count_peers_ipv4 = 0u64;
                        for (peer_id, torrent_peer) in t.seeds.iter() {
                            if count_seeds_ipv4 == amount {
                                break;
                            }
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => {
                                    if (amount != 0 && count_seeds_ipv4 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.seeds_ipv4.insert(*peer_id, torrent_peer.clone());
                                    count_seeds_ipv4 += 1;
                                }
                                SocketAddr::V6(_) => {}
                            }
                        }
                        for (peer_id, torrent_peer) in t.peers.iter() {
                            if count_peers_ipv4 == amount {
                                break;
                            }
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => {
                                    if (amount != 0 && count_peers_ipv4 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.peers_ipv4.insert(*peer_id, torrent_peer.clone());
                                    count_peers_ipv4 += 1;
                                }
                                SocketAddr::V6(_) => {}
                            }
                        }
                    }
                    TorrentPeersType::IPv6 => {
                        let mut count_seeds_ipv6 = 0u64;
                        let mut count_peers_ipv6 = 0u64;
                        for (peer_id, torrent_peer) in t.seeds.iter() {
                            if count_seeds_ipv6 == amount {
                                break;
                            }
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => {}
                                SocketAddr::V6(_) => {
                                    if (amount != 0 && count_seeds_ipv6 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.seeds_ipv6.insert(*peer_id, torrent_peer.clone());
                                    count_seeds_ipv6 += 1;
                                }
                            }
                        }
                        for (peer_id, torrent_peer) in t.peers.iter() {
                            if count_peers_ipv6 == amount {
                                break;
                            }
                            match torrent_peer.peer_addr {
                                SocketAddr::V4(_) => {}
                                SocketAddr::V6(_) => {
                                    if (amount != 0 && count_peers_ipv6 == amount) || (self_ip.is_some() && torrent_peer.peer_addr.ip() != self_ip.unwrap()) {
                                        continue;
                                    }
                                    torrent_peers.peers_ipv6.insert(*peer_id, torrent_peer.clone());
                                    count_peers_ipv6 += 1;
                                }
                            }
                        }
                    }
                }
                torrent_peers
            }
        }
    }

    pub async fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let map = self.torrents_map.clone();
        let mut lock = map.write();
        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                let mut completed_count = 0u64;
                if completed && persistent {
                    completed_count += 1;
                }
                let mut torrent = TorrentEntry {
                    seeds: BTreeMap::new(),
                    peers: BTreeMap::new(),
                    completed: completed_count,
                    updated: std::time::Instant::now()
                };
                match torrent_peer.left {
                    NumberOfBytes(0) => {
                        let seeds_count = torrent.seeds.len();
                        torrent.seeds.insert(peer_id, torrent_peer);
                        if completed {
                            self.add_torrents_update(info_hash, completed_count as i64).await;
                        }
                        if seeds_count != torrent.seeds.len() {
                            self.update_stats(StatsEvent::Seeds, (torrent.seeds.len() - seeds_count) as i64).await;
                        }
                    }
                    _ => {
                        let peers_count = torrent.peers.len();
                        torrent.peers.insert(peer_id, torrent_peer);
                        if peers_count != torrent.peers.len() {
                            self.update_stats(StatsEvent::Peers, (torrent.peers.len() - peers_count) as i64).await;
                        }
                    }
                }
                v.insert(torrent.clone());
                self.update_stats(StatsEvent::Torrents, 1).await;
                torrent
            }
            Entry::Occupied(mut t) => {
                let torrent = t.get_mut();
                if completed && persistent {
                    torrent.completed += 1;
                }
                torrent.updated = std::time::Instant::now();
                match torrent_peer.left {
                    NumberOfBytes(0) => {
                        let seeds_count = torrent.seeds.len();
                        torrent.seeds.insert(peer_id, torrent_peer);
                        if completed {
                            self.add_torrents_update(info_hash, torrent.completed as i64).await;
                        }
                        if seeds_count != torrent.seeds.len() {
                            self.update_stats(StatsEvent::Seeds, (torrent.seeds.len() - seeds_count) as i64).await;
                        }
                    }
                    _ => {
                        let peers_count = torrent.peers.len();
                        torrent.peers.insert(peer_id, torrent_peer);
                        if peers_count != torrent.peers.len() {
                            self.update_stats(StatsEvent::Peers, (torrent.peers.len() - peers_count) as i64).await;
                        }
                    }
                }
                TorrentEntry {
                    seeds: torrent.seeds.clone(),
                    peers: torrent.peers.clone(),
                    completed: torrent.completed,
                    updated: torrent.updated
                }
            }
        }
    }

    pub async fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool) -> Option<(TorrentEntry, bool, bool)>
    {
        let map = self.torrents_map.clone();
        let mut lock = map.write();
        match lock.entry(info_hash) {
            Entry::Vacant(_) => { None }
            Entry::Occupied(mut t) => {
                let mut torrent = t.get_mut();
                let seed_found = match torrent.seeds.remove(&peer_id) {
                    None => { false }
                    Some(_) => { true }
                };
                let peer_found = match torrent.peers.remove(&peer_id) {
                    None => { false}
                    Some(_) => { true }
                };
                if seed_found {
                    self.update_stats(StatsEvent::Seeds, -1).await;
                }
                if peer_found {
                    self.update_stats(StatsEvent::Peers, -1).await;
                }
                let torrent_return = torrent.clone();
                if persistent {
                    self.remove_torrents_update(info_hash).await;
                } else {
                    if torrent.peers.len() == 0 && torrent.seeds.len() == 0 {
                        t.remove();
                        self.update_stats(StatsEvent::Torrents, -1).await;
                    }
                }
                Some((TorrentEntry {
                    seeds: torrent_return.seeds.clone(),
                    peers: torrent_return.peers.clone(),
                    completed: torrent_return.completed,
                    updated: torrent_return.updated
                }, seed_found, peer_found))
            }
        }
    }

    pub async fn torrent_peers_cleanup(&self, peer_timeout: Duration, persistent: bool)
    {
        let lock = self.torrents_map.clone();
        let mut start = 0u64;
        let mut amount = self.config.cleanup_chunks.unwrap_or(100000);
        let mut seeds_found = 0u64;
        let mut peers_found = 0u64;
        loop {
            for (info_hash, torrent_entry) in self.get_torrents_chunk(start as usize, amount as usize).await.iter() {
                if start > amount {
                    break;
                }
                for (peer_id, torrent_peer) in self.get_torrent_peers(*info_hash, 0, TorrentPeersType::All, None).await.seeds_ipv4 {
                    if torrent_peer.updated.elapsed() > peer_timeout {
                        match self.remove_torrent_peer(*info_hash, peer_id, persistent).await {
                            None => {}
                            Some((_, seeds, peers)) => {
                                if seeds {
                                    seeds_found += 1;
                                    self.update_stats(StatsEvent::Seeds, -1).await;
                                }
                                if peers {
                                    peers_found += 1;
                                    self.update_stats(StatsEvent::Peers, -1).await;
                                }
                            }
                        }
                    }
                }
                for (peer_id, torrent_peer) in self.get_torrent_peers(*info_hash, 0, TorrentPeersType::All, None).await.seeds_ipv6 {
                    if torrent_peer.updated.elapsed() > peer_timeout {
                        match self.remove_torrent_peer(*info_hash, peer_id, persistent).await {
                            None => {}
                            Some((_, seeds, peers)) => {
                                if seeds {
                                    seeds_found += 1;
                                    self.update_stats(StatsEvent::Seeds, -1).await;
                                }
                                if peers {
                                    peers_found += 1;
                                    self.update_stats(StatsEvent::Peers, -1).await;
                                }
                            }
                        }
                    }
                }
                for (peer_id, torrent_peer) in self.get_torrent_peers(*info_hash, 0, TorrentPeersType::All, None).await.peers_ipv4 {
                    if torrent_peer.updated.elapsed() > peer_timeout {
                        match self.remove_torrent_peer(*info_hash, peer_id, persistent).await {
                            None => {}
                            Some((_, seeds, peers)) => {
                                if seeds {
                                    seeds_found += 1;
                                    self.update_stats(StatsEvent::Seeds, -1).await;
                                }
                                if peers {
                                    peers_found += 1;
                                    self.update_stats(StatsEvent::Peers, -1).await;
                                }
                            }
                        }
                    }
                }
                for (peer_id, torrent_peer) in self.get_torrent_peers(*info_hash, 0, TorrentPeersType::All, None).await.peers_ipv6 {
                    if torrent_peer.updated.elapsed() > peer_timeout {
                        match self.remove_torrent_peer(*info_hash, peer_id, persistent).await {
                            None => {}
                            Some((_, seeds, peers)) => {
                                if seeds {
                                    seeds_found += 1;
                                    self.update_stats(StatsEvent::Seeds, -1).await;
                                }
                                if peers {
                                    peers_found += 1;
                                    self.update_stats(StatsEvent::Peers, -1).await;
                                }
                            }
                        }
                    }
                }
            }
            start += amount;
        }
        info!("[PEERS CLEANUP] Removed {} seeds and {} peers", seeds_found, peers_found);
    }
}
