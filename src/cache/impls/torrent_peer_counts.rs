use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;

impl TorrentPeerCounts {
   #[inline]
    pub fn total_seeds(&self) -> u64 {
        self.bt_seeds_ipv4 + self.bt_seeds_ipv6 + self.rtc_seeds
    }

    #[inline]
    pub fn total_peers(&self) -> u64 {
        self.bt_peers_ipv4 + self.bt_peers_ipv6 + self.rtc_peers
    }
}