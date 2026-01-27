use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub struct CleanupStats {
    pub torrents: AtomicU64,
    pub seeds: AtomicU64,
    pub peers: AtomicU64,
}

impl CleanupStats {
    pub(crate) fn new() -> Self {
        Self {
            torrents: AtomicU64::new(0),
            seeds: AtomicU64::new(0),
            peers: AtomicU64::new(0),
        }
    }

    pub(crate) fn add_torrents(&self, n: u64) {
        self.torrents.fetch_add(n, Ordering::Relaxed);
    }

    pub(crate) fn add_seeds(&self, n: u64) {
        self.seeds.fetch_add(n, Ordering::Relaxed);
    }

    pub(crate) fn add_peers(&self, n: u64) {
        self.peers.fetch_add(n, Ordering::Relaxed);
    }

    pub(crate) fn apply_to_tracker(&self, tracker: &Arc<TorrentTracker>) {
        let torrents = self.torrents.swap(0, Ordering::Relaxed);
        let seeds = self.seeds.swap(0, Ordering::Relaxed);
        let peers = self.peers.swap(0, Ordering::Relaxed);

        
        if torrents > 0 {
            tracker.update_stats(StatsEvent::Torrents, -(torrents as i64));
        }
        if seeds > 0 {
            tracker.update_stats(StatsEvent::Seeds, -(seeds as i64));
        }
        if peers > 0 {
            tracker.update_stats(StatsEvent::Peers, -(peers as i64));
        }
    }
}