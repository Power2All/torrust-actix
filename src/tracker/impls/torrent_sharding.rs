use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::info;
use parking_lot::RwLock;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio_shutdown::Shutdown;
use crate::common::common::shutdown_waiting;
use crate::tracker::structs::cleanup_stats_atomic::CleanupStatsAtomic;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub const CACHE_LINE_SIZE: usize = 64;

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

        // Configuration for thread groups instead of individual threads
        // This provides a balance between parallelism and resource usage
        const SHARDS_PER_THREAD: usize = 16; // Each thread handles 16 shards (256/16 = 16 threads)
        const NUM_THREAD_GROUPS: usize = 256 / SHARDS_PER_THREAD;

        let mut cleanup_handles = Vec::with_capacity(NUM_THREAD_GROUPS);

        // Create dedicated cleanup tasks for groups of shards
        for group_idx in 0..NUM_THREAD_GROUPS {
            let start_shard = group_idx * SHARDS_PER_THREAD;
            let end_shard = start_shard + SHARDS_PER_THREAD;

            let torrent_tracker_clone = Arc::clone(&torrent_tracker);
            let shutdown_clone = shutdown.clone();
            let self_shards = self.shards.clone();

            // Spawn a dedicated task for this group of shards
            let handle: JoinHandle<()> = tokio::spawn(async move {
                loop {
                    // Wait for interval or shutdown
                    if shutdown_waiting(
                        Duration::from_secs(cleanup_interval),
                        shutdown_clone.clone()
                    ).await {
                        break;
                    }

                    let stats = Arc::new(CleanupStatsAtomic::new());
                    let cutoff = Instant::now() - peer_timeout;

                    // Process this thread's assigned shards
                    for shard_idx in start_shard..end_shard {
                        Self::cleanup_shard_dedicated(
                            Arc::clone(&torrent_tracker_clone),
                            &self_shards,
                            shard_idx as u8,
                            cutoff,
                            persistent,
                            Arc::clone(&stats)
                        ).await;
                    }

                    // Apply stats for this group
                    stats.apply_to_tracker(&torrent_tracker_clone);
                }

                info!("Cleanup thread group {} shutting down", group_idx);
            });

            cleanup_handles.push(handle);
        }

        // Wait for shutdown signal
        shutdown.handle().await;

        // Cancel all cleanup tasks
        for handle in cleanup_handles {
            handle.abort();
            let _ = handle.await;
        }
    }

    // Optimized cleanup for dedicated thread model
    async fn cleanup_shard_dedicated(
        torrent_tracker: Arc<TorrentTracker>,
        shards: &[Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>; 256],
        shard_idx: u8,
        cutoff: Instant,
        persistent: bool,
        stats: Arc<CleanupStatsAtomic>
    ) {
        let (mut torrents_removed, mut seeds_removed, mut peers_removed) = (0u64, 0u64, 0u64);

        let shard_arc = &shards[shard_idx as usize];

        // Use a two-phase approach: identify then modify
        // This minimizes lock hold time
        let mut expired_full: Vec<InfoHash> = Vec::new();
        let mut expired_partial: Vec<(InfoHash, Vec<PeerId>, Vec<PeerId>)> = Vec::new();

        // Phase 1: Quick read to identify expired entries
        {
            let shard_read = shard_arc.read();

            if shard_read.is_empty() {
                return;
            }

            // Pre-allocate based on shard size estimate
            expired_full.reserve(shard_read.len() / 10);
            expired_partial.reserve(shard_read.len() / 5);

            for (info_hash, torrent_entry) in shard_read.iter() {
                // Fast path: entire torrent is stale
                if torrent_entry.updated < cutoff {
                    expired_full.push(*info_hash);
                    continue;
                }

                // Check for expired peers
                let mut expired_seeds = Vec::new();
                let mut expired_peers = Vec::new();

                // Use iterator chaining for efficiency
                for (peer_id, peer) in &torrent_entry.seeds {
                    if peer.updated < cutoff {
                        expired_seeds.push(*peer_id);
                    }
                }

                for (peer_id, peer) in &torrent_entry.peers {
                    if peer.updated < cutoff {
                        expired_peers.push(*peer_id);
                    }
                }

                if !expired_seeds.is_empty() || !expired_peers.is_empty() {
                    expired_partial.push((*info_hash, expired_seeds, expired_peers));
                }
            }
        }

        // Phase 2: Apply modifications if needed
        if !expired_partial.is_empty() || !expired_full.is_empty() {
            let mut shard_write = shard_arc.write();

            // Process partial expirations
            for (info_hash, expired_seeds, expired_peers) in expired_partial {
                if let Entry::Occupied(mut entry) = shard_write.entry(info_hash) {
                    let torrent_entry = entry.get_mut();

                    // Remove expired seeds
                    for peer_id in expired_seeds {
                        if torrent_entry.seeds.remove(&peer_id).is_some() {
                            seeds_removed += 1;
                        }
                    }

                    // Remove expired peers
                    for peer_id in expired_peers {
                        if torrent_entry.peers.remove(&peer_id).is_some() {
                            peers_removed += 1;
                        }
                    }

                    // Remove empty torrent if not persistent
                    if !persistent && torrent_entry.seeds.is_empty() && torrent_entry.peers.is_empty() {
                        entry.remove();
                        torrents_removed += 1;
                    }
                }
            }

            // Process full expirations
            for info_hash in expired_full {
                if let Entry::Occupied(entry) = shard_write.entry(info_hash) {
                    // Double-check staleness (defensive programming)
                    if entry.get().updated >= cutoff {
                        continue;
                    }

                    if persistent {
                        // Keep torrent but clear peers
                        let torrent_entry = entry.into_mut();
                        seeds_removed += torrent_entry.seeds.len() as u64;
                        peers_removed += torrent_entry.peers.len() as u64;
                        torrent_entry.seeds.clear();
                        torrent_entry.peers.clear();
                    } else {
                        // Remove entire torrent
                        let torrent_entry = entry.get();
                        seeds_removed += torrent_entry.seeds.len() as u64;
                        peers_removed += torrent_entry.peers.len() as u64;
                        entry.remove();
                        torrents_removed += 1;
                    }
                }
            }
        }

        // Update stats atomically
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
            info!("[PEERS] Shard: {shard_idx} - Torrents: {torrents_removed} - Seeds: {seeds_removed} - Peers: {peers_removed}");
        }
    }

    // Alternative implementation with true per-shard threads (use with caution)
    pub async fn cleanup_threads_per_shard(&self, torrent_tracker: Arc<TorrentTracker>, shutdown: Shutdown, peer_timeout: Duration, persistent: bool) {
        let cleanup_interval = torrent_tracker.config.tracker_config.peers_cleanup_interval;
        let mut cleanup_handles = Vec::with_capacity(256);

        // Create 256 dedicated tasks - one per shard
        for shard_idx in 0u8..=255u8 {
            let torrent_tracker_clone = Arc::clone(&torrent_tracker);
            let shutdown_clone = shutdown.clone();
            let shard_arc = Arc::clone(&self.shards[shard_idx as usize]);

            let handle: JoinHandle<()> = tokio::spawn(async move {
                // Use exponential backoff for empty shards to reduce CPU usage
                let mut empty_cycles = 0u32;

                loop {
                    // Adaptive interval based on shard activity
                    let wait_duration = if empty_cycles > 0 {
                        // Exponential backoff for empty shards (up to 10x normal interval)
                        Duration::from_secs(cleanup_interval * std::cmp::min(10, 2_u64.pow(empty_cycles)))
                    } else {
                        Duration::from_secs(cleanup_interval)
                    };

                    if shutdown_waiting(wait_duration, shutdown_clone.clone()).await {
                        break;
                    }

                    let stats = CleanupStatsAtomic::new();
                    let cutoff = Instant::now() - peer_timeout;

                    let (torrents_removed, seeds_removed, peers_removed) =
                        Self::cleanup_single_shard(&shard_arc, cutoff, persistent);

                    // Track if shard was empty
                    if torrents_removed == 0 && seeds_removed == 0 && peers_removed == 0 {
                        empty_cycles = std::cmp::min(empty_cycles + 1, 5);
                    } else {
                        empty_cycles = 0;

                        // Update global stats
                        stats.add_torrents(torrents_removed);
                        stats.add_seeds(seeds_removed);
                        stats.add_peers(peers_removed);
                        stats.apply_to_tracker(&torrent_tracker_clone);

                        info!("[PEERS] Shard: {shard_idx} - Torrents: {torrents_removed} - Seeds: {seeds_removed} - Peers: {peers_removed}");
                    }
                }
            });

            cleanup_handles.push(handle);
        }

        // Wait for shutdown
        shutdown.handle().await;

        // Cancel all tasks
        for handle in cleanup_handles {
            handle.abort();
            let _ = handle.await;
        }
    }

    // Simplified cleanup for single shard (used by per-shard threads)
    fn cleanup_single_shard(
        shard: &Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>,
        cutoff: Instant,
        persistent: bool
    ) -> (u64, u64, u64) {
        let mut torrents_removed = 0u64;
        let mut seeds_removed = 0u64;
        let mut peers_removed = 0u64;

        let mut shard_write = shard.write();

        let mut to_remove = Vec::new();

        for (info_hash, torrent_entry) in shard_write.iter_mut() {
            if torrent_entry.updated < cutoff {
                // Entire torrent is stale
                seeds_removed += torrent_entry.seeds.len() as u64;
                peers_removed += torrent_entry.peers.len() as u64;

                if persistent {
                    torrent_entry.seeds.clear();
                    torrent_entry.peers.clear();
                } else {
                    to_remove.push(*info_hash);
                    torrents_removed += 1;
                }
            } else {
                // Check individual peers
                let old_seeds = torrent_entry.seeds.len();
                let old_peers = torrent_entry.peers.len();

                torrent_entry.seeds.retain(|_, peer| peer.updated >= cutoff);
                torrent_entry.peers.retain(|_, peer| peer.updated >= cutoff);

                seeds_removed += (old_seeds - torrent_entry.seeds.len()) as u64;
                peers_removed += (old_peers - torrent_entry.peers.len()) as u64;

                if !persistent && torrent_entry.seeds.is_empty() && torrent_entry.peers.is_empty() {
                    to_remove.push(*info_hash);
                    torrents_removed += 1;
                }
            }
        }

        // Remove empty torrents
        for info_hash in to_remove {
            shard_write.remove(&info_hash);
        }

        (torrents_removed, seeds_removed, peers_removed)
    }

    // Keep all existing methods unchanged for compatibility
    #[tracing::instrument(level = "debug")]
    #[inline(always)]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool {
        let shard_index = info_hash.0[0] as usize;
        unsafe {
            self.shards.get_unchecked(shard_index)
                .read()
                .contains_key(&info_hash)
        }
    }

    #[tracing::instrument(level = "debug")]
    #[inline(always)]
    pub fn contains_peer(&self, info_hash: InfoHash, peer_id: PeerId) -> bool {
        let shard_index = info_hash.0[0] as usize;
        unsafe {
            let shard = self.shards.get_unchecked(shard_index).read();
            shard.get(&info_hash)
                .map(|entry| entry.seeds.contains_key(&peer_id) || entry.peers.contains_key(&peer_id))
                .unwrap_or(false)
        }
    }

    #[tracing::instrument(level = "debug")]
    #[inline(always)]
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
        let total_size: usize = self.shards.iter()
            .map(|shard| shard.read().len())
            .sum();

        let mut torrents_return = BTreeMap::new();

        if total_size < 100000 {
            torrents_return = BTreeMap::new();
        }

        for shard in &self.shards {
            let shard_data = shard.read();
            torrents_return.extend(shard_data.iter().map(|(k, v)| (*k, v.clone())));
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
            let shard_idx = info_hash.0[0] as usize;
            shard_groups[shard_idx].push(info_hash);
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
            let shard_idx = info_hash.0[0] as usize;
            shard_groups[shard_idx].push(idx);
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

    pub fn par_iter_all_torrents<F>(&self, f: F)
    where
        F: Fn(&InfoHash, &TorrentEntry) + Sync + Send
    {
        use rayon::prelude::*;

        self.shards.par_iter().for_each(|shard| {
            let shard_data = shard.read();
            for (k, v) in shard_data.iter() {
                f(k, v);
            }
        });
    }
}