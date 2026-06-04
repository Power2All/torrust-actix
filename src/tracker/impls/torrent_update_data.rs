use crate::tracker::structs::announce_entry::AnnounceEntry;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;

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

impl From<&AnnounceEntry> for TorrentUpdateData {
    fn from(entry: &AnnounceEntry) -> Self {
        TorrentUpdateData {
            seeds_ipv4: entry.counts.seeds_ipv4 as u64,
            seeds_ipv6: entry.counts.seeds_ipv6 as u64,
            peers_ipv4: entry.counts.peers_ipv4 as u64,
            peers_ipv6: entry.counts.peers_ipv6 as u64,
            rtc_seeds: entry.rtc_seeds.len() as u64,
            rtc_peers: entry.rtc_peers.len() as u64,
            completed: entry.completed,
        }
    }
}