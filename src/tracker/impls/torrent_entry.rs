use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::types::peer_map::PeerMap;

impl TorrentEntry {
    /// Creates an empty torrent entry with the update timestamp set to now.
    pub fn new() -> Self {
        TorrentEntry {
            seeds: PeerMap::default(),
            seeds_ipv6: PeerMap::default(),
            peers: PeerMap::default(),
            peers_ipv6: PeerMap::default(),
            rtc_seeds: PeerMap::default(),
            rtc_peers: PeerMap::default(),
            completed: 0,
            updated: std::time::Instant::now(),
        }
    }
}

impl Default for TorrentEntry {
    fn default() -> Self {
        Self::new()
    }
}