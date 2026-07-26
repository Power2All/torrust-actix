#[derive(Clone, Copy, Debug)]
pub struct TorrentCounts {
    pub seeds_ipv4: usize,
    pub seeds_ipv6: usize,
    pub peers_ipv4: usize,
    pub peers_ipv6: usize,
    /// RtcTorrent seeders, counted separately because RtcTorrent responses report only the
    /// WebRTC side of the swarm.
    pub rtc_seeds: usize,
    /// RtcTorrent leechers.
    pub rtc_peers: usize,
    pub completed: u64,
}