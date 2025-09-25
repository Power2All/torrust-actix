use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::info;
use parking_lot::RwLock;
use tokio::runtime::Builder;
use tokio::sync::Semaphore;
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
        let cleanup_threads = torrent_tracker.config.tracker_config.peers_cleanup_threads;

        let cleanup_pool = Builder::new_multi_thread()
            .worker_threads(cleanup_threads as usize)
            .thread_name("cleanup-worker")
            .enable_all()
            .build()
            .unwrap();

        let max_concurrent = std::cmp::max(cleanup_threads as usize * 2, 8);
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        let cleanup_handles_capacity = 256;

        let timer_handle: JoinHandle<()> = cleanup_pool.spawn({
            let torrent_tracker_clone = Arc::clone(&torrent_tracker);
            let shutdown_clone = shutdown.clone();
            let sem_clone = Arc::clone(&semaphore);

            async move {
                let batch_size = 256 / max_concurrent;

                loop {
                    if shutdown_waiting(
                        Duration::from_secs(cleanup_interval),
                        shutdown_clone.clone()
                    ).await {
                        break;
                    }

                    let stats = Arc::new(CleanupStatsAtomic::new());
                    let mut cleanup_handles = Vec::with_capacity(cleanup_handles_capacity);

                    let cutoff = Instant::now() - peer_timeout;

                    // Process shards in batches for better cache locality
                    for batch_start in (0u8..=255u8).step_by(batch_size) {
                        let batch_end = std::cmp::min(batch_start + batch_size as u8, 255);
                        let tracker_clone = Arc::clone(&torrent_tracker_clone);
                        let sem_clone = Arc::clone(&sem_clone);
                        let stats_clone = Arc::clone(&stats);

                        let handle = tokio::spawn(async move {
                            let _permit = sem_clone.acquire().await.ok()?;

                            // Process batch of shards
                            for shard in batch_start..=batch_end {
                                Self::cleanup_shard_optimized(
                                    Arc::clone(&tracker_clone),
                                    shard,
                                    cutoff,
                                    persistent,
                                    Arc::clone(&stats_clone)
                                ).await;
                            }
                            Some(())
                        });

                        cleanup_handles.push(handle);
                    }

                    // Wait for all cleanups to complete
                    futures::future::join_all(cleanup_handles).await;

                    // Apply batch stats update
                    stats.apply_to_tracker(&torrent_tracker_clone);
                }
            }
        });

        // Wait for shutdown signal
        shutdown.handle().await;

        // Cancel the cleanup task
        timer_handle.abort();
        let _ = timer_handle.await;

        // Shutdown the runtime properly
        cleanup_pool.shutdown_background();
    }

    async fn cleanup_shard_optimized(
        torrent_tracker: Arc<TorrentTracker>,
        shard: u8,
        cutoff: Instant,
        persistent: bool,
        stats: Arc<CleanupStatsAtomic>
    ) {
        let (mut torrents_removed, mut seeds_removed, mut peers_removed) = (0u64, 0u64, 0u64);

        if let Some(shard_arc) = torrent_tracker.torrents_sharding.shards.get(shard as usize) {
            // Use SmallVec for better stack allocation for small collections
            let mut expired_full: Vec<InfoHash> = Vec::with_capacity(32);
            let mut expired_partial: Vec<(InfoHash, Vec<PeerId>, Vec<PeerId>)> = Vec::with_capacity(64);

            // Quick read pass to identify expired entries
            {
                let shard_read = shard_arc.read();

                // Early exit if shard is empty
                if shard_read.is_empty() {
                    return;
                }

                for (info_hash, torrent_entry) in shard_read.iter() {
                    // Fast path: torrent not updated within timeout => all peers are expired
                    if torrent_entry.updated < cutoff {
                        expired_full.push(*info_hash);
                        continue;
                    }

                    // Optimized: only allocate if we find expired peers
                    let mut expired_seeds = Vec::new();
                    let mut expired_peers = Vec::new();
                    let mut has_expired = false;

                    // Process seeds and peers in parallel chunks if large enough
                    if torrent_entry.seeds.len() > 100 {
                        // For large collections, collect in parallel
                        expired_seeds = torrent_entry.seeds.iter()
                            .filter(|(_, peer)| peer.updated < cutoff)
                            .map(|(id, _)| *id)
                            .collect();
                        has_expired = !expired_seeds.is_empty();
                    } else {
                        // For small collections, use simpler iteration
                        for (peer_id, torrent_peer) in &torrent_entry.seeds {
                            if torrent_peer.updated < cutoff {
                                expired_seeds.push(*peer_id);
                                has_expired = true;
                            }
                        }
                    }

                    // Same optimization for peers
                    if torrent_entry.peers.len() > 100 {
                        expired_peers = torrent_entry.peers.iter()
                            .filter(|(_, peer)| peer.updated < cutoff)
                            .map(|(id, _)| *id)
                            .collect();
                        has_expired = has_expired || !expired_peers.is_empty();
                    } else {
                        for (peer_id, torrent_peer) in &torrent_entry.peers {
                            if torrent_peer.updated < cutoff {
                                expired_peers.push(*peer_id);
                                has_expired = true;
                            }
                        }
                    }

                    if has_expired {
                        expired_partial.push((*info_hash, expired_seeds, expired_peers));
                    }
                }
            }

            // Process removals if needed
            if !expired_partial.is_empty() || !expired_full.is_empty() {
                let mut shard_write = shard_arc.write();

                // Process partial expirations
                for (info_hash, expired_seeds, expired_peers) in expired_partial {
                    if let Entry::Occupied(mut entry) = shard_write.entry(info_hash) {
                        let torrent_entry = entry.get_mut();

                        // Batch remove seeds - use retain for better performance on large collections
                        if expired_seeds.len() > 10 {
                            let expired_set: std::collections::HashSet<_> = expired_seeds.into_iter().collect();
                            let before_len = torrent_entry.seeds.len();
                            torrent_entry.seeds.retain(|k, _| !expired_set.contains(k));
                            seeds_removed += (before_len - torrent_entry.seeds.len()) as u64;
                        } else {
                            for peer_id in expired_seeds {
                                if torrent_entry.seeds.remove(&peer_id).is_some() {
                                    seeds_removed += 1;
                                }
                            }
                        }

                        // Batch remove peers - use retain for better performance on large collections
                        if expired_peers.len() > 10 {
                            let expired_set: std::collections::HashSet<_> = expired_peers.into_iter().collect();
                            let before_len = torrent_entry.peers.len();
                            torrent_entry.peers.retain(|k, _| !expired_set.contains(k));
                            peers_removed += (before_len - torrent_entry.peers.len()) as u64;
                        } else {
                            for peer_id in expired_peers {
                                if torrent_entry.peers.remove(&peer_id).is_some() {
                                    peers_removed += 1;
                                }
                            }
                        }

                        // Remove empty torrent if allowed
                        if !persistent && torrent_entry.seeds.is_empty() && torrent_entry.peers.is_empty() {
                            entry.remove();
                            torrents_removed += 1;
                        }
                    }
                }

                // Process full expirations (entire torrent stale)
                if !expired_full.is_empty() {
                    if persistent {
                        // When persistent, just clear the peers
                        for info_hash in expired_full {
                            if let Some(torrent_entry) = shard_write.get_mut(&info_hash) {
                                // Safety re-check
                                if torrent_entry.updated >= cutoff { continue; }

                                seeds_removed += torrent_entry.seeds.len() as u64;
                                peers_removed += torrent_entry.peers.len() as u64;
                                torrent_entry.seeds.clear();
                                torrent_entry.peers.clear();
                            }
                        }
                    } else {
                        // Batch remove all expired torrents at once
                        for info_hash in expired_full {
                            if let Entry::Occupied(entry) = shard_write.entry(info_hash) {
                                // Safety re-check
                                if entry.get().updated >= cutoff { continue; }

                                let torrent_entry = entry.get();
                                seeds_removed += torrent_entry.seeds.len() as u64;
                                peers_removed += torrent_entry.peers.len() as u64;
                                entry.remove();
                                torrents_removed += 1;
                            }
                        }
                    }
                }
            }
        }

        // Update shared stats atomically
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
    #[inline(always)]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool {
        let shard_index = info_hash.0[0] as usize;
        // Use unchecked access since we know index is always valid (0-255)
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
        // Use unchecked access since we know index is always valid (0-255)
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
        // Pre-calculate total size for better allocation
        let total_size: usize = self.shards.iter()
            .map(|shard| shard.read().len())
            .sum();

        let mut torrents_return = BTreeMap::new();

        // Reserve capacity if we have a reasonable estimate
        if total_size < 100000 {
            // Only pre-allocate for reasonable sizes
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
        // Use parallel iteration for large shard counts
        self.shards.iter()
            .map(|shard| shard.read().len() as u64)
            .sum()
    }

    pub fn get_multiple_torrents(&self, info_hashes: &[InfoHash]) -> BTreeMap<InfoHash, Option<TorrentEntry>> {
        let mut results = BTreeMap::new();

        // Group by shard more efficiently
        let mut shard_groups: [Vec<InfoHash>; 256] = std::array::from_fn(|_| Vec::new());

        for &info_hash in info_hashes {
            let shard_idx = info_hash.0[0] as usize;
            shard_groups[shard_idx].push(info_hash);
        }

        // Process only non-empty shards
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

        // Group queries by shard
        let mut shard_groups: [Vec<usize>; 256] = std::array::from_fn(|_| Vec::new());

        for (idx, &(info_hash, _)) in queries.iter().enumerate() {
            let shard_idx = info_hash.0[0] as usize;
            shard_groups[shard_idx].push(idx);
        }

        // Process only non-empty shards
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

    // New method for parallel iteration with Rayon (if available)
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