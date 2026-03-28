#[derive(Debug, Clone, Default)]
pub struct TorrentPeerCounts {
    pub bt_seeds_ipv4: u64,
    pub bt_seeds_ipv6: u64,
    pub rtc_seeds: u64,
    pub bt_peers_ipv4: u64,
    pub bt_peers_ipv6: u64,
    pub rtc_peers: u64,
    pub completed: u64,
}