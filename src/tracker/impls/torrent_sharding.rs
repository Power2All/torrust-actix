use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use log::info;
use parking_lot::RwLock;
use tokio::runtime::Builder;
use tokio_shutdown::Shutdown;
use crate::common::common::shutdown_waiting;
use crate::stats::enums::stats_event::StatsEvent;
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
        // Cache config values to avoid repeated access
        let cleanup_interval = torrent_tracker.config.tracker_config.peers_cleanup_interval;
        let cleanup_threads = torrent_tracker.config.tracker_config.peers_cleanup_threads;

        // Create shared thread pool for actual cleanup work
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
                    .enable_all()
                    .build()
                    .unwrap()
            }
        });

        // Single timer thread that schedules work
        let timer_handle = {
            let torrent_tracker_clone = Arc::clone(&torrent_tracker);
            let shutdown_clone = shutdown.clone();
            let pool_clone = Arc::clone(&cleanup_pool);

            tokio::spawn(async move {
                loop {
                    if shutdown_waiting(
                        Duration::from_secs(cleanup_interval),
                        shutdown_clone.clone()
                    ).await {
                        return;
                    }

                    // Spawn cleanup tasks for all shards (maintaining exact same behavior)
                    let mut cleanup_handles = Vec::new();

                    for shard in 0u8..=255u8 {
                        let tracker_clone = Arc::clone(&torrent_tracker_clone);

                        let handle = pool_clone.spawn(async move {
                            Self::cleanup_shard(tracker_clone, shard, peer_timeout, persistent).await
                        });

                        cleanup_handles.push(handle);
                    }

                    // Wait for all cleanup tasks to complete (maintains synchronous behavior)
                    for handle in cleanup_handles {
                        let _ = handle.await; // Ignore join errors
                    }
                }
            })
        };

        shutdown.clone().handle().await;
        timer_handle.abort();
        mem::forget(cleanup_pool);
    }

    async fn cleanup_shard(
        torrent_tracker: Arc<TorrentTracker>,
        shard: u8,
        peer_timeout: Duration,
        persistent: bool
    ) {
        let (mut torrents_removed, mut seeds_removed, mut peers_removed) = (0u64, 0u64, 0u64);

        // Get shard reference once and reuse it - EXACT same logic as original
        if let Some(shard_arc) = torrent_tracker.torrents_sharding.get_shard(shard) {
            // Collect expired entries first with minimal locking - EXACT same logic
            let expired_entries = {
                let shard_read = shard_arc.read();
                let mut expired = Vec::new();

                for (info_hash, torrent_entry) in shard_read.iter() {
                    let mut expired_seeds = Vec::new();
                    let mut expired_peers = Vec::new();

                    // Check seeds for expiration - EXACT same logic
                    for (peer_id, torrent_peer) in &torrent_entry.seeds {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            expired_seeds.push(*peer_id);
                        }
                    }

                    // Check peers for expiration - EXACT same logic
                    for (peer_id, torrent_peer) in &torrent_entry.peers {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            expired_peers.push(*peer_id);
                        }
                    }

                    if !expired_seeds.is_empty() || !expired_peers.is_empty() {
                        expired.push((*info_hash, expired_seeds, expired_peers));
                    }
                }
                expired
            };

            // Process expired entries with write lock - EXACT same logic
            if !expired_entries.is_empty() {
                let mut shard_write = shard_arc.write();

                for (info_hash, expired_seeds, expired_peers) in expired_entries {
                    if let Entry::Occupied(mut entry) = shard_write.entry(info_hash) {
                        let torrent_entry = entry.get_mut();

                        // Remove expired seeds - EXACT same logic
                        for peer_id in expired_seeds {
                            if torrent_entry.seeds.remove(&peer_id).is_some() {
                                seeds_removed += 1;
                            }
                        }

                        // Remove expired peers - EXACT same logic
                        for peer_id in expired_peers {
                            if torrent_entry.peers.remove(&peer_id).is_some() {
                                peers_removed += 1;
                            }
                        }

                        // Remove empty torrent entry if not persistent - EXACT same logic
                        if !persistent && torrent_entry.seeds.is_empty() && torrent_entry.peers.is_empty() {
                            entry.remove();
                            torrents_removed += 1;
                        }
                    }
                }
            }
        }

        // Batch update stats to minimize contention - EXACT same logic
        if seeds_removed > 0 {
            torrent_tracker.update_stats(StatsEvent::Seeds, -(seeds_removed as i64));
        }
        if peers_removed > 0 {
            torrent_tracker.update_stats(StatsEvent::Peers, -(peers_removed as i64));
        }
        if torrents_removed > 0 {
            torrent_tracker.update_stats(StatsEvent::Torrents, -(torrents_removed as i64));
        }

        if seeds_removed > 0 || peers_removed > 0 || torrents_removed > 0 {
            info!("[PEERS] Shard: {shard} - Torrents: {torrents_removed} - Seeds: {seeds_removed} - Peers: {peers_removed}");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool {
        let shard_index = info_hash.0[0] as usize;
        self.shards[shard_index].read().contains_key(&info_hash)
    }

    #[tracing::instrument(level = "debug")]
    pub fn contains_peer(&self, info_hash: InfoHash, peer_id: PeerId) -> bool {
        let shard_index = info_hash.0[0] as usize;
        let shard = self.shards[shard_index].read();

        match shard.get(&info_hash) {
            Some(torrent_entry) => {
                torrent_entry.seeds.contains_key(&peer_id) ||
                    torrent_entry.peers.contains_key(&peer_id)
            }
            None => false,
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_shard(&self, shard: u8) -> Option<Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>> {
        self.shards.get(shard as usize).cloned()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_shard_content(&self, shard: u8) -> BTreeMap<InfoHash, TorrentEntry> {
        match self.shards.get(shard as usize) {
            Some(shard) => shard.read_recursive().clone(),
            None => BTreeMap::new(),
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_all_content(&self) -> BTreeMap<InfoHash, TorrentEntry> {
        let mut torrents_return = BTreeMap::new();
        for shard in &self.shards {
            let mut shard_data = shard.read_recursive().clone();
            torrents_return.append(&mut shard_data);
        }
        torrents_return
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_torrents_amount(&self) -> u64 {
        let mut torrents = 0u64;
        for shard in &self.shards {
            torrents += shard.read_recursive().len() as u64;
        }
        torrents
    }

    pub fn get_multiple_torrents(&self, info_hashes: &[InfoHash]) -> BTreeMap<InfoHash, Option<TorrentEntry>> {
        let mut shard_groups: BTreeMap<u8, Vec<InfoHash>> = BTreeMap::new();
        for &info_hash in info_hashes {
            shard_groups.entry(info_hash.0[0]).or_default().push(info_hash);
        }

        let mut results = BTreeMap::new();
        for (shard_index, hashes) in shard_groups {
            let shard = self.shards[shard_index as usize].read();
            for hash in hashes {
                results.insert(hash, shard.get(&hash).cloned());
            }
        }
        results
    }

    pub fn batch_contains_peers(&self, queries: &[(InfoHash, PeerId)]) -> Vec<bool> {
        let mut shard_groups: BTreeMap<u8, Vec<(InfoHash, PeerId)>> = BTreeMap::new();
        for &query in queries {
            shard_groups.entry(query.0.0[0]).or_default().push(query);
        }

        let mut results = Vec::with_capacity(queries.len());
        let mut query_results: BTreeMap<(InfoHash, PeerId), bool> = BTreeMap::new();

        for (shard_index, shard_queries) in shard_groups {
            let shard = self.shards[shard_index as usize].read();
            for &(info_hash, peer_id) in &shard_queries {
                let contains = match shard.get(&info_hash) {
                    Some(entry) => entry.seeds.contains_key(&peer_id) || entry.peers.contains_key(&peer_id),
                    None => false,
                };
                query_results.insert((info_hash, peer_id), contains);
            }
        }

        for &query in queries {
            results.push(query_results[&query]);
        }
        results
    }}