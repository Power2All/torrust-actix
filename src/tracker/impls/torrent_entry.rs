use std::collections::BTreeMap;
use crate::tracker::structs::torrent_entry::TorrentEntry;

impl TorrentEntry {
    pub fn new() -> TorrentEntry {
        TorrentEntry {
            peers: BTreeMap::new(),
            seeds: BTreeMap::new(),
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
