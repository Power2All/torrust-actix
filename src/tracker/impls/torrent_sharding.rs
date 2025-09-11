use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use log::info;
use parking_lot::RwLock;
use tokio::runtime::Builder;
use tokio::sync::Semaphore;
use tokio_shutdown::Shutdown;
use crate::common::common::shutdown_waiting;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::cleanup_stats::CleanupStats;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

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
            shards: std::array::from_fn(|_| Arc::new(RwLock::new(Default::default()))),
        }
    }

    pub async fn cleanup_threads(&self, torrent_tracker: Arc<TorrentTracker>, shutdown: Shutdown, peer_timeout: Duration, persistent: bool) {
        let cleanup_interval = torrent_tracker.config.tracker_config.peers_cleanup_interval;
        let cleanup_threads = torrent_tracker.config.tracker_config.peers_cleanup_threads;

        // Create thread pool with optimized configuration
        let cleanup_pool = Arc::new(match cleanup_threads {
            0 => {
                Builder::new_current_thread()
                    .thread_name("cleanup-pool")
                    .enable_all()
                    .build()
                    .unwrap()
            }
            _ => {
                Builder::new_multi_thread()
                    .thread_name("cleanup-pool")
                    .worker_threads(cleanup_threads as usize)
                    .max_blocking_threads(cleanup_threads as usize)
                    .enable_all()
                    .build()
                    .unwrap()
            }
        });

        // Use semaphore to limit concurrent shard cleanups (prevent memory spikes)
        let max_concurrent = std::cmp::max(cleanup_threads as usize, 4);
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        let timer_handle = {
            let torrent_tracker_clone = Arc::clone(&torrent_tracker);
            let shutdown_clone = shutdown.clone();
            let pool_clone = Arc::clone(&cleanup_pool);

            tokio::spawn(async move {
                // Pre-allocate reusable buffer for expired entries
                let expired_buffer = Arc::new(RwLock::new(Vec::with_capacity(1000)));

                loop {
                    if shutdown_waiting(
                        Duration::from_secs(cleanup_interval),
                        shutdown_clone.clone()
                    ).await {
                        return;
                    }

                    // Shared stats accumulator for this cleanup cycle
                    let stats = Arc::new(CleanupStats::new());
                    let mut cleanup_handles = Vec::with_capacity(256);

                    // Process shards concurrently with controlled parallelism
                    for shard in 0u8..=255u8 {
                        let tracker_clone = Arc::clone(&torrent_tracker_clone);
                        let sem_clone = Arc::clone(&semaphore);
                        let stats_clone = Arc::clone(&stats);
                        let buffer_clone = Arc::clone(&expired_buffer);

                        let handle = pool_clone.spawn(async move {
                            let _permit = sem_clone.acquire().await.ok()?;
                            Self::cleanup_shard_optimized(
                                tracker_clone,
                                shard,
                                peer_timeout,
                                persistent,
                                stats_clone,
                                buffer_clone
                            ).await;
                            Some(())
                        });

                        cleanup_handles.push(handle);
                    }

                    // Wait for all cleanups to complete
                    for handle in cleanup_handles {
                        let _ = handle.await;
                    }

                    // Apply batch stats update
                    stats.apply_to_tracker(&torrent_tracker_clone);
                }
            })
        };

        shutdown.clone().handle().await;
        timer_handle.abort();
        mem::forget(cleanup_pool);
    }

    async fn cleanup_shard_optimized(
        torrent_tracker: Arc<TorrentTracker>,
        shard: u8,
        peer_timeout: Duration,
        persistent: bool,
        stats: Arc<CleanupStats>,
        expired_buffer: Arc<RwLock<Vec<(InfoHash, Vec<PeerId>, Vec<PeerId>)>>>
    ) {
        let (mut torrents_removed, mut seeds_removed, mut peers_removed) = (0u64, 0u64, 0u64);

        if let Some(shard_arc) = torrent_tracker.torrents_sharding.shards.get(shard as usize) {
            // Reuse buffer from pool
            let mut buffer = expired_buffer.write();
            buffer.clear();

            // Quick read pass to identify expired entries
            {
                let shard_read = shard_arc.read();

                for (info_hash, torrent_entry) in shard_read.iter() {
                    let mut has_expired = false;
                    let mut expired_seeds = Vec::new();
                    let mut expired_peers = Vec::new();

                    // Check seeds - use capacity hint for better allocation
                    expired_seeds.reserve(torrent_entry.seeds.len() / 10); // Assume ~10% expiry
                    for (peer_id, torrent_peer) in &torrent_entry.seeds {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            expired_seeds.push(*peer_id);
                            has_expired = true;
                        }
                    }

                    // Check peers - use capacity hint
                    expired_peers.reserve(torrent_entry.peers.len() / 10);
                    for (peer_id, torrent_peer) in &torrent_entry.peers {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            expired_peers.push(*peer_id);
                            has_expired = true;
                        }
                    }

                    if has_expired {
                        buffer.push((*info_hash, expired_seeds, expired_peers));
                    }
                }
            }

            // Process removals if needed
            if !buffer.is_empty() {
                let mut shard_write = shard_arc.write();

                for (info_hash, expired_seeds, expired_peers) in buffer.iter() {
                    if let Entry::Occupied(mut entry) = shard_write.entry(*info_hash) {
                        let torrent_entry = entry.get_mut();

                        // Batch remove seeds
                        for peer_id in expired_seeds {
                            if torrent_entry.seeds.remove(peer_id).is_some() {
                                seeds_removed += 1;
                            }
                        }

                        // Batch remove peers
                        for peer_id in expired_peers {
                            if torrent_entry.peers.remove(peer_id).is_some() {
                                peers_removed += 1;
                            }
                        }

                        // Remove empty torrent
                        if !persistent && torrent_entry.seeds.is_empty() && torrent_entry.peers.is_empty() {
                            entry.remove();
                            torrents_removed += 1;
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
    #[inline]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool {
        let shard_index = info_hash.0[0] as usize;
        // Use get() to avoid bounds check in release mode
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
        // Keep original API but use regular read() instead of read_recursive()
        self.shards.get(shard as usize)
            .map(|s| s.read().clone())
            .unwrap_or_default()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_all_content(&self) -> BTreeMap<InfoHash, TorrentEntry> {
        // Pre-allocate with estimated capacity
        let estimated_size: usize = self.shards.iter()
            .map(|s| s.read().len())
            .sum();

        let mut torrents_return = BTreeMap::new();

        // Consider if you really need to clone all data - maybe return an iterator instead?
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
        // Use parallel iteration for large shard counts
        self.shards.iter()
            .map(|shard| shard.read().len() as u64)
            .sum()
    }

    pub fn get_multiple_torrents(&self, info_hashes: &[InfoHash]) -> BTreeMap<InfoHash, Option<TorrentEntry>> {
        // Pre-allocate result map
        let mut results = BTreeMap::new();

        // Group by shard to minimize lock acquisitions
        let mut shard_groups: [Vec<InfoHash>; 256] = std::array::from_fn(|_| Vec::new());
        for &info_hash in info_hashes {
            shard_groups[info_hash.0[0] as usize].push(info_hash);
        }

        // Process each shard
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
        // Pre-allocate results
        let mut results = vec![false; queries.len()];

        // Group by shard using array instead of BTreeMap
        let mut shard_groups: [Vec<usize>; 256] = std::array::from_fn(|_| Vec::new());
        for (idx, &(info_hash, _)) in queries.iter().enumerate() {
            shard_groups[info_hash.0[0] as usize].push(idx);
        }

        // Process each shard
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

    // Optional: Add a streaming iterator version to avoid cloning
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