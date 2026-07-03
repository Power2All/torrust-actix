use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;

impl TorrentPeerCounts {
    /// Returns the total seeds across IPv4, IPv6 and RTC.
   #[inline]
    pub fn total_seeds(&self) -> u64 {
        self.bt_seeds_ipv4 + self.bt_seeds_ipv6 + self.rtc_seeds
    }

    /// Returns the total leechers across IPv4, IPv6 and RTC.
    #[inline]
    pub fn total_peers(&self) -> u64 {
        self.bt_peers_ipv4 + self.bt_peers_ipv6 + self.rtc_peers
    }
}