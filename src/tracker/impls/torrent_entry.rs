use std::sync::atomic::AtomicU64;
use crate::tracker::structs::torrent_entry::TorrentEntry;

impl TorrentEntry {
    pub fn new() -> TorrentEntry {
        TorrentEntry {
            peers: AtomicU64::new(0),
            seeds: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            updated: std::time::Instant::now(),
        }
    }
}

impl Default for TorrentEntry {
    fn default() -> Self {
        Self::new()
    }
}
