use crate::tracker::structs::torrent_entry::{AHashMap, TorrentEntry};

impl TorrentEntry {
    #[inline]
    pub fn new() -> TorrentEntry {
        TorrentEntry {
            peers: AHashMap::default(),
            seeds: AHashMap::default(),
            completed: 0u64,
            updated: std::time::Instant::now(),
        }
    }
}

impl Default for TorrentEntry {
    fn default() -> Self {
        Self::new()
    }
}