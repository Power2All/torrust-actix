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