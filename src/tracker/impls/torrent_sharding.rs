use crate::common::common::shutdown_waiting;
use crate::tracker::structs::cleanup_stats::CleanupStats;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::info;
use parking_lot::RwLock;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;
use tokio_shutdown::Shutdown;

impl Default for TorrentSharding {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl TorrentSharding {
    #[tracing::instrument(level = "debug")]
    pub fn new() -> TorrentSharding {
        TorrentSharding {
            shards: std::array::from_fn(|_| Arc::new(RwLock::new(BTreeMap::new()))),
        }
    }

    pub async fn cleanup_threads(&self, torrent_tracker: Arc<TorrentTracker>, shutdown: Shutdown, peer_timeout: Duration, persistent: bool) {
        let cleanup_interval = torrent_tracker.config.tracker_config.peers_cleanup_interval;
        let cleanup_threads = torrent_tracker.config.tracker_config.peers_cleanup_threads;
        let cleanup_pool = Builder::new_multi_thread()
            .worker_threads(cleanup_threads as usize)
            .thread_name("cleanup-worker")
            .enable_all()
            .build()
            .unwrap();
        let timer_handle: JoinHandle<()> = cleanup_pool.spawn({
            let torrent_tracker_clone = Arc::clone(&torrent_tracker);
            let shutdown_clone = shutdown.clone();
            async move {
                loop {
                    if shutdown_waiting(
                        Duration::from_secs(cleanup_interval),
                        shutdown_clone.clone()
                    ).await {
                        break;
                    }
                    let stats = Arc::new(CleanupStats::new());
                    let mut cleanup_handles = Vec::with_capacity(cleanup_threads as usize);
                    let shards_per_thread = 256 / cleanup_threads as usize;
                    let remainder = 256 % cleanup_threads as usize;
                    for thread_idx in 0..cleanup_threads as usize {
                        let tracker_clone = Arc::clone(&torrent_tracker_clone);
                        let stats_clone = Arc::clone(&stats);
                        let start_shard = thread_idx * shards_per_thread + thread_idx.min(remainder);
                        let extra = if thread_idx < remainder { 1 } else { 0 };
                        let end_shard = start_shard + shards_per_thread + extra;
                        let handle = tokio::spawn(async move {
                            for shard in start_shard..end_shard {
                                Self::cleanup_shard_optimized(
                                    tracker_clone.clone(),
                                    shard as u8,
                                    peer_timeout,
                                    persistent,
                                    stats_clone.clone()
                                ).await;
                            }
                        });
                        cleanup_handles.push(handle);
                    }
                    for handle in cleanup_handles {
                        let _ = handle.await;
                    }
                    stats.apply_to_tracker(&torrent_tracker_clone);
                }
            }
        });
        shutdown.handle().await;
        timer_handle.abort();
        let _ = timer_handle.await;
        cleanup_pool.shutdown_background();
    }
    
    async fn cleanup_shard_optimized(
        torrent_tracker: Arc<TorrentTracker>,
        shard: u8,
        peer_timeout: Duration,
        persistent: bool,
        stats: Arc<CleanupStats>
    ) {
        let (mut torrents_removed, mut seeds_removed, mut peers_removed) = (0u64, 0u64, 0u64);
        if let Some(shard_arc) = torrent_tracker.torrents_sharding.shards.get(shard as usize) {
            let cutoff = std::time::Instant::now() - peer_timeout;
            let mut expired_full: Vec<InfoHash> = Vec::new();
            let mut expired_partial: Vec<(InfoHash, Vec<PeerId>, Vec<PeerId>)> = Vec::new();
            {
                let shard_read = shard_arc.read();
                for (info_hash, torrent_entry) in shard_read.iter() {
                    if torrent_entry.updated < cutoff {
                        expired_full.push(*info_hash);
                        continue;
                    }
                    let mut has_expired = false;
                    let mut expired_seeds = Vec::new();
                    let mut expired_peers = Vec::new();
                    expired_seeds.reserve(torrent_entry.seeds.len() / 10);
                    for (peer_id, torrent_peer) in &torrent_entry.seeds {
                        if torrent_peer.updated < cutoff {
                            expired_seeds.push(*peer_id);
                            has_expired = true;
                        }
                    }
                    expired_peers.reserve(torrent_entry.peers.len() / 10);
                    for (peer_id, torrent_peer) in &torrent_entry.peers {
                        if torrent_peer.updated < cutoff {
                            expired_peers.push(*peer_id);
                            has_expired = true;
                        }
                    }
                    if has_expired {
                        expired_partial.push((*info_hash, expired_seeds, expired_peers));
                    }
                }
            }
            if !expired_partial.is_empty() || !expired_full.is_empty() {
                let mut shard_write = shard_arc.write();
                for (info_hash, expired_seeds, expired_peers) in expired_partial.iter() {
                    if let Entry::Occupied(mut entry) = shard_write.entry(*info_hash) {
                        let torrent_entry = entry.get_mut();
                        for peer_id in expired_seeds {
                            if torrent_entry.seeds.remove(peer_id).is_some() {
                                seeds_removed += 1;
                            }
                        }
                        for peer_id in expired_peers {
                            if torrent_entry.peers.remove(peer_id).is_some() {
                                peers_removed += 1;
                            }
                        }
                        if !persistent && torrent_entry.seeds.is_empty() && torrent_entry.peers.is_empty() {
                            entry.remove();
                            torrents_removed += 1;
                        }
                    }
                }
                for info_hash in expired_full.iter() {
                    if let Entry::Occupied(entry) = shard_write.entry(*info_hash) {
                        let mut entry = entry;
                        if entry.get().updated >= cutoff { continue; }
                        let torrent_entry = entry.get_mut();
                        let seeds_len = torrent_entry.seeds.len() as u64;
                        let peers_len = torrent_entry.peers.len() as u64;
                        if !persistent {
                            entry.remove();
                            torrents_removed += 1;
                            seeds_removed += seeds_len;
                            peers_removed += peers_len;
                        } else {
                            if seeds_len > 0 {
                                torrent_entry.seeds.clear();
                                seeds_removed += seeds_len;
                            }
                            if peers_len > 0 {
                                torrent_entry.peers.clear();
                                peers_removed += peers_len;
                            }
                        }
                    }
                }
            }
        }
        if torrents_removed > 0 {
            stats.add_torrents(torrents_removed);
        }
        if seeds_removed > 0 {
            stats.add_seeds(seeds_removed);
        }
        if peers_removed > 0 {
            stats.add_peers(peers_removed);
        }
        if seeds_removed > 0 || peers_removed > 0 || torrents_removed > 0 {
            info!("[PEERS] Shard: {shard} - Torrents: {torrents_removed} - Seeds: {seeds_removed} - Peers: {peers_removed}");
        }
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool {
        let shard_index = info_hash.0[0] as usize;
        self.shards.get(shard_index)
            .map(|s| s.read().contains_key(&info_hash))
            .unwrap_or(false)
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn contains_peer(&self, info_hash: InfoHash, peer_id: PeerId) -> bool {
        let shard_index = info_hash.0[0] as usize;
        self.shards.get(shard_index)
            .and_then(|s| {
                let shard = s.read();
                shard.get(&info_hash).map(|entry| {
                    entry.seeds.contains_key(&peer_id) || entry.peers.contains_key(&peer_id)
                })
            })
            .unwrap_or(false)
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn get_shard(&self, shard: u8) -> Option<Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>> {
        self.shards.get(shard as usize).cloned()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_shard_content(&self, shard: u8) -> BTreeMap<InfoHash, TorrentEntry> {
        self.shards.get(shard as usize)
            .map(|s| s.read().clone())
            .unwrap_or_default()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_all_content(&self) -> BTreeMap<InfoHash, TorrentEntry> {
        let mut torrents_return = BTreeMap::new();
        for shard in &self.shards {
            let shard_data = shard.read();
            for (k, v) in shard_data.iter() {
                torrents_return.insert(*k, v.clone());
            }
        }
        torrents_return
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_torrents_amount(&self) -> u64 {
        self.shards.iter()
            .map(|shard| shard.read().len() as u64)
            .sum()
    }

    pub fn get_multiple_torrents(&self, info_hashes: &[InfoHash]) -> BTreeMap<InfoHash, Option<TorrentEntry>> {
        let mut results = BTreeMap::new();
        let mut shard_groups: [Vec<InfoHash>; 256] = std::array::from_fn(|_| Vec::new());
        for &info_hash in info_hashes {
            shard_groups[info_hash.0[0] as usize].push(info_hash);
        }
        for (shard_index, hashes) in shard_groups.iter().enumerate() {
            if !hashes.is_empty() {
                let shard = self.shards[shard_index].read();
                for &hash in hashes {
                    results.insert(hash, shard.get(&hash).cloned());
                }
            }
        }
        results
    }

    pub fn batch_contains_peers(&self, queries: &[(InfoHash, PeerId)]) -> Vec<bool> {
        let mut results = vec![false; queries.len()];
        let mut shard_groups: [Vec<usize>; 256] = std::array::from_fn(|_| Vec::new());
        for (idx, &(info_hash, _)) in queries.iter().enumerate() {
            shard_groups[info_hash.0[0] as usize].push(idx);
        }
        for (shard_index, indices) in shard_groups.iter().enumerate() {
            if !indices.is_empty() {
                let shard = self.shards[shard_index].read();
                for &idx in indices {
                    let (info_hash, peer_id) = queries[idx];
                    results[idx] = shard.get(&info_hash)
                        .map(|entry| entry.seeds.contains_key(&peer_id) || entry.peers.contains_key(&peer_id))
                        .unwrap_or(false);
                }
            }
        }
        results
    }

    pub fn iter_all_torrents<F>(&self, mut f: F)
    where
        F: FnMut(&InfoHash, &TorrentEntry)
    {
        for shard in &self.shards {
            let shard_data = shard.read();
            for (k, v) in shard_data.iter() {
                f(k, v);
            }
        }
    }
}