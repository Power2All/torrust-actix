use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::types::ahash_map::AHashMap;

impl Default for TorrentEntry {
    fn default() -> Self {
        TorrentEntry {
            seeds: AHashMap::default(),
            peers: AHashMap::default(),
            rtc_seeds: AHashMap::default(),
            rtc_peers: AHashMap::default(),
            completed: 0,
            updated: std::time::Instant::now(),
        }
    }
}