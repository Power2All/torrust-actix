use std::collections::btree_map::Entry;
use std::net::{IpAddr, SocketAddr};
use log::info;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::{AHashMap, TorrentEntry};
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_peers::TorrentPeers;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub fn get_torrent_peers(&self, info_hash: InfoHash, amount: usize, ip_type: TorrentPeersType, self_ip: Option<IpAddr>) -> Option<TorrentPeers>
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
                    returned_data.seeds_ipv4 = self.get_peers(&data.seeds, TorrentPeersType::IPv4, self_ip, amount);
                    returned_data.seeds_ipv6 = self.get_peers(&data.seeds, TorrentPeersType::IPv6, self_ip, amount);
                    returned_data.peers_ipv4 = self.get_peers(&data.peers, TorrentPeersType::IPv4, self_ip, amount);
                    returned_data.peers_ipv6 = self.get_peers(&data.peers, TorrentPeersType::IPv6, self_ip, amount);
                }
                TorrentPeersType::IPv4 => {
                    returned_data.seeds_ipv4 = self.get_peers(&data.seeds, TorrentPeersType::IPv4, self_ip, amount);
                    returned_data.peers_ipv4 = self.get_peers(&data.peers, TorrentPeersType::IPv4, self_ip, amount);
                }
                TorrentPeersType::IPv6 => {
                    returned_data.seeds_ipv6 = self.get_peers(&data.seeds, TorrentPeersType::IPv6, self_ip, amount);
                    returned_data.peers_ipv6 = self.get_peers(&data.peers, TorrentPeersType::IPv6, self_ip, amount);
                }
            }

            returned_data
        })
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn get_peers(&self, peers: &AHashMap<PeerId, TorrentPeer>, type_ip: TorrentPeersType, self_ip: Option<IpAddr>, amount: usize) -> AHashMap<PeerId, TorrentPeer>
    {
        let should_include = |peer_addr: &SocketAddr| -> bool {
            let ip_type_match = match type_ip {
                TorrentPeersType::All => return false,
                TorrentPeersType::IPv4 => peer_addr.is_ipv4(),
                TorrentPeersType::IPv6 => peer_addr.is_ipv6(),
            };

            ip_type_match && self_ip.is_none_or(|ip| ip != peer_addr.ip())
        };

        
        let mut result = AHashMap::default();
        result.reserve(amount.min(peers.len()));

        for (peer_id, torrent_peer) in peers.iter() {

            if amount != 0 && result.len() >= amount {
                break;
            }

            if should_include(&torrent_peer.peer_addr) {
                result.insert(*peer_id, torrent_peer.clone());
            }
        }

        result
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool) -> (Option<TorrentEntry>, TorrentEntry)
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();

        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                let mut torrent_entry = TorrentEntry {
                    seeds: AHashMap::default(),
                    peers: AHashMap::default(),
                    completed: 0,
                    updated: std::time::Instant::now()
                };

                if completed && torrent_peer.left == NumberOfBytes(0) {
                    self.update_stats(StatsEvent::Completed, 1);
                    torrent_entry.completed = 1;
                }

                self.update_stats(StatsEvent::Torrents, 1);

                if torrent_peer.left == NumberOfBytes(0) {
                    self.update_stats(StatsEvent::Seeds, 1);
                    torrent_entry.seeds.insert(peer_id, torrent_peer);
                } else {
                    self.update_stats(StatsEvent::Peers, 1);
                    torrent_entry.peers.insert(peer_id, torrent_peer);
                }

                let entry_clone = torrent_entry.clone();
                v.insert(torrent_entry);
                (None, entry_clone)
            }
            Entry::Occupied(mut o) => {
                let previous_torrent = o.get().clone();
                let entry = o.get_mut();

                let was_seed = entry.seeds.remove(&peer_id).is_some();
                let was_peer = entry.peers.remove(&peer_id).is_some();

                if was_seed {
                    self.update_stats(StatsEvent::Seeds, -1);
                }
                if was_peer {
                    self.update_stats(StatsEvent::Peers, -1);
                }

                if completed {
                    self.update_stats(StatsEvent::Completed, 1);
                    entry.completed += 1;
                }

                if torrent_peer.left == NumberOfBytes(0) {
                    self.update_stats(StatsEvent::Seeds, 1);
                    entry.seeds.insert(peer_id, torrent_peer);
                } else {
                    self.update_stats(StatsEvent::Peers, 1);
                    entry.peers.insert(peer_id, torrent_peer);
                }

                entry.updated = std::time::Instant::now();

                (Some(previous_torrent), entry.clone())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
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

                let was_seed = entry.seeds.remove(&peer_id).is_some();
                let was_peer = entry.peers.remove(&peer_id).is_some();

                if was_seed {
                    self.update_stats(StatsEvent::Seeds, -1);
                }
                if was_peer {
                    self.update_stats(StatsEvent::Peers, -1);
                }

                if !persistent && entry.seeds.is_empty() && entry.peers.is_empty() {
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