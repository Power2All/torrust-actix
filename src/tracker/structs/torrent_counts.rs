#[derive(Clone, Copy, Debug)]
pub struct TorrentCounts {
    pub seeds_ipv4: usize,
    pub seeds_ipv6: usize,
    pub peers_ipv4: usize,
    pub peers_ipv6: usize,
    pub completed: u64,
}