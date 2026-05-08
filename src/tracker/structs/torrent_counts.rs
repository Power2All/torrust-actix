use crate::tracker::structs::torrent_entry::TorrentEntry;

#[derive(Clone, Copy, Debug)]
pub struct TorrentCounts {
    pub seeds_ipv4: usize,
    pub seeds_ipv6: usize,
    pub peers_ipv4: usize,
    pub peers_ipv6: usize,
    pub completed: u64,
}

impl TorrentCounts {
    pub fn from_entry(entry: &TorrentEntry) -> Self {
        Self {
            seeds_ipv4: entry.seeds.len(),
            seeds_ipv6: entry.seeds_ipv6.len(),
            peers_ipv4: entry.peers.len(),
            peers_ipv6: entry.peers_ipv6.len(),
            completed: entry.completed,
        }
    }

    #[inline]
    pub fn total_seeds(&self) -> usize {
        self.seeds_ipv4 + self.seeds_ipv6
    }

    #[inline]
    pub fn total_peers(&self) -> usize {
        self.peers_ipv4 + self.peers_ipv6
    }
}