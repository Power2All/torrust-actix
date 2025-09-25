use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use log::info;
use crate::tracker::impls::torrent_sharding::CACHE_LINE_SIZE;
use crate::tracker::structs::cleanup_stats_atomic::CleanupStatsAtomic;
use crate::tracker::structs::padded_atomic_u64::PaddedAtomicU64;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl CleanupStatsAtomic {
    pub(crate) fn new() -> Self {
        Self {
            torrents: PaddedAtomicU64 {
                value: AtomicU64::new(0),
                _padding: [0; CACHE_LINE_SIZE - std::mem::size_of::<AtomicU64>()],
            },
            seeds: PaddedAtomicU64 {
                value: AtomicU64::new(0),
                _padding: [0; CACHE_LINE_SIZE - std::mem::size_of::<AtomicU64>()],
            },
            peers: PaddedAtomicU64 {
                value: AtomicU64::new(0),
                _padding: [0; CACHE_LINE_SIZE - std::mem::size_of::<AtomicU64>()],
            },
        }
    }

    pub(crate) fn add_torrents(&self, count: u64) {
        self.torrents.value.fetch_add(count, Ordering::Relaxed);
    }

    pub(crate) fn add_seeds(&self, count: u64) {
        self.seeds.value.fetch_add(count, Ordering::Relaxed);
    }

    pub(crate) fn add_peers(&self, count: u64) {
        self.peers.value.fetch_add(count, Ordering::Relaxed);
    }

    pub(crate) fn apply_to_tracker(&self, _tracker: &Arc<TorrentTracker>) {
        let torrents = self.torrents.value.load(Ordering::Relaxed);
        let seeds = self.seeds.value.load(Ordering::Relaxed);
        let peers = self.peers.value.load(Ordering::Relaxed);

        if torrents > 0 || seeds > 0 || peers > 0 {
            info!("[CLEANUP TOTAL] Torrents: {torrents} - Seeds: {seeds} - Peers: {peers}");
        }
    }
}