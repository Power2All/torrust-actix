use crate::tracker::structs::torrent_entry::TorrentEntry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TorrentUpdateData {
    pub seeds_ipv4: u64,
    pub seeds_ipv6: u64,
    pub peers_ipv4: u64,
    pub peers_ipv6: u64,
    pub rtc_seeds: u64,
    pub rtc_peers: u64,
    pub completed: u64,
}

impl From<&TorrentEntry> for TorrentUpdateData {
    fn from(entry: &TorrentEntry) -> Self {
        TorrentUpdateData {
            seeds_ipv4: entry.seeds.len() as u64,
            seeds_ipv6: entry.seeds_ipv6.len() as u64,
            peers_ipv4: entry.peers.len() as u64,
            peers_ipv6: entry.peers_ipv6.len() as u64,
            rtc_seeds: entry.rtc_seeds.len() as u64,
            rtc_peers: entry.rtc_peers.len() as u64,
            completed: entry.completed,
        }
    }
}