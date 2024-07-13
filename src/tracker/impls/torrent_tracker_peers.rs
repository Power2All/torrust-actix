use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use concat_arrays::concat_arrays;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn get_torrent_peers(&self, info_hash: InfoHash) -> (HashMap<PeerId, TorrentPeer>, HashMap<PeerId, TorrentPeer>)
    {
        let mut return_data_seeds: HashMap<PeerId, TorrentPeer> = HashMap::new();
        let return_data_peers = HashMap::new();
        let seeds_map = self.seeds_map.clone();
        let peers_map = self.peers_map.clone();
        let start_range: [u8; 40] = concat_arrays!(info_hash.0, [0; 20]);
        let end_range: [u8; 40] = concat_arrays!(info_hash.0, [255; 20]);
        for seed in seeds_map.range(start_range..=end_range) {
            return_data_seeds.insert(PeerId(*seed.key().last_chunk::<20>().unwrap()), seed.value().clone());
        }
        for peer in peers_map.range(start_range..=end_range) {
            return_data_seeds.insert(PeerId(*peer.key().last_chunk::<20>().unwrap()), peer.value().clone());
        }
        (return_data_seeds, return_data_peers)
    }

    pub async fn add_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let torrents_map = self.torrents_map.clone();
        let torrent = match torrents_map.get(&info_hash) {
            None => { TorrentEntry::new() }
            Some(torrent) => { TorrentEntry {
                seeds: AtomicU64::new(torrent.value().seeds.load(Ordering::SeqCst)),
                peers: AtomicU64::new(torrent.value().peers.load(Ordering::SeqCst)),
                completed: AtomicU64::new(torrent.value().completed.load(Ordering::SeqCst)),
                updated: std::time::Instant::now()
            }}
        };
        let seeds_map = self.seeds_map.clone();
        let peers_map = self.peers_map.clone();
        let hash_info_peer_id: [u8; 40] = concat_arrays!(info_hash.0, peer_id.0);
        let start_range: [u8; 40] = concat_arrays!(info_hash.0, [0; 20]);
        let end_range: [u8; 40] = concat_arrays!(info_hash.0, [255; 20]);
        let mut seeds = 0u64;
        let mut peers = 0u64;
        match torrent_peer.left {
            NumberOfBytes(0) => {
                if completed {
                    torrent.completed.fetch_add(1, Ordering::SeqCst);
                    self.update_stats(StatsEvent::Completed, 1).await;
                    if persistent {
                        self.add_torrents_update(info_hash, torrent.completed.load(Ordering::SeqCst) as i64).await
                    }
                }
                seeds_map.insert(hash_info_peer_id, torrent_peer);
                peers_map.remove(&hash_info_peer_id);
                let _: Vec<_> = seeds_map.range(start_range..=end_range).inspect(|_| seeds += 1).collect();
                let _: Vec<_> = peers_map.range(start_range..=end_range).inspect(|_| peers += 1).collect();
            }
            _ => {
                peers_map.insert(hash_info_peer_id, torrent_peer);
                seeds_map.remove(&hash_info_peer_id);
                let _: Vec<_> = seeds_map.range(start_range..=end_range).inspect(|_| seeds += 1).collect();
                let _: Vec<_> = peers_map.range(start_range..=end_range).inspect(|_| peers += 1).collect();
            }
        }
        let _ = torrent.seeds.fetch_sub(torrent.seeds.load(Ordering::SeqCst), Ordering::SeqCst);
        let _ = torrent.seeds.fetch_add(seeds, Ordering::SeqCst);
        let _ = torrent.peers.fetch_sub(torrent.peers.load(Ordering::SeqCst), Ordering::SeqCst);
        let _ = torrent.peers.fetch_add(peers, Ordering::SeqCst);
        torrents_map.insert(info_hash, TorrentEntry {
            seeds: AtomicU64::new(torrent.seeds.load(Ordering::SeqCst)),
            peers: AtomicU64::new(torrent.peers.load(Ordering::SeqCst)),
            completed: AtomicU64::new(torrent.completed.load(Ordering::SeqCst)),
            updated: std::time::Instant::now()
        });
        TorrentEntry {
            seeds: AtomicU64::new(torrent.seeds.load(Ordering::SeqCst)),
            peers: AtomicU64::new(torrent.peers.load(Ordering::SeqCst)),
            completed: AtomicU64::new(torrent.completed.load(Ordering::SeqCst)),
            updated: std::time::Instant::now()
        }
    }

    pub async fn remove_torrent_peer(&self, info_hash: InfoHash, peer_id: PeerId, persistent: bool) -> Option<TorrentEntry>
    {
        let torrents_map = self.torrents_map.clone();
        let torrent = match torrents_map.get(&info_hash) {
            None => { TorrentEntry::new() }
            Some(torrent) => { TorrentEntry {
                seeds: AtomicU64::new(torrent.value().seeds.load(Ordering::SeqCst)),
                peers: AtomicU64::new(torrent.value().peers.load(Ordering::SeqCst)),
                completed: AtomicU64::new(torrent.value().completed.load(Ordering::SeqCst)),
                updated: std::time::Instant::now()
            }}
        };
        let seeds_map = self.seeds_map.clone();
        let peers_map = self.peers_map.clone();
        let hash_info_peer_id: [u8; 40] = concat_arrays!(info_hash.0, peer_id.0);
        let start_range: [u8; 40] = concat_arrays!(info_hash.0, [0; 20]);
        let end_range: [u8; 40] = concat_arrays!(info_hash.0, [255; 20]);
        let mut seeds = 0u64;
        let mut peers = 0u64;
        seeds_map.remove(&hash_info_peer_id);
        peers_map.remove(&hash_info_peer_id);
        let _: Vec<_> = seeds_map.range(start_range..=end_range).inspect(|_| seeds += 1).collect();
        let _: Vec<_> = peers_map.range(start_range..=end_range).inspect(|_| peers += 1).collect();
        if !persistent && seeds == 0 && peers == 0 {
            torrents_map.remove(&info_hash);
        } else {
            torrents_map.insert(info_hash, TorrentEntry {
                seeds: AtomicU64::new(torrent.seeds.load(Ordering::SeqCst)),
                peers: AtomicU64::new(torrent.peers.load(Ordering::SeqCst)),
                completed: AtomicU64::new(torrent.completed.load(Ordering::SeqCst)),
                updated: std::time::Instant::now(),
            });
        }
        Some(TorrentEntry {
            seeds: AtomicU64::new(torrent.seeds.load(Ordering::SeqCst)),
            peers: AtomicU64::new(torrent.peers.load(Ordering::SeqCst)),
            completed: AtomicU64::new(torrent.completed.load(Ordering::SeqCst)),
            updated: std::time::Instant::now(),
        })
    }

    pub async fn torrent_peers_cleanup(&self, peer_timeout: Duration, persistent: bool)
    {
        let mut removed_seeds = 0u64;
        let mut removed_peers = 0u64;
        let seeds_map = self.seeds_map.clone();
        let peers_map = self.peers_map.clone();
        for seed in seeds_map.iter() {
            if seed.value().updated.elapsed() > peer_timeout {
                self.remove_torrent_peer(InfoHash(*seed.key().first_chunk::<20>().unwrap()), PeerId(*seed.key().last_chunk::<20>().unwrap()), persistent).await;
                removed_seeds += 1;
            }
        }
        for peer in peers_map.iter() {
            if peer.value().updated.elapsed() > peer_timeout {
                self.remove_torrent_peer(InfoHash(*peer.key().first_chunk::<20>().unwrap()), PeerId(*peer.key().last_chunk::<20>().unwrap()), persistent).await;
                removed_peers += 1;
            }
        }
        info!("[PEERS CLEANUP] Removed {} seeds and {} peers", removed_seeds, removed_peers);
    }
}
