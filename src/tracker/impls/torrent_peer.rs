use crate::tracker::structs::torrent_peer::TorrentPeer;
use std::net::{
    IpAddr,
    SocketAddr
};

impl TorrentPeer {
    pub fn peer_addr_from_ip_and_port_and_opt_host_ip(remote_ip: IpAddr, port: u16) -> SocketAddr {
        SocketAddr::new(remote_ip, port)
    }

    pub fn is_rtctorrent(&self) -> bool {
        self.rtc_data.is_some()
    }

    pub fn rtc_sdp_offer(&self) -> Option<String> {
        self.rtc_data.as_ref()?.sdp_offer.as_ref().map(|cb| cb.decompress())
    }

    pub fn rtc_sdp_answer(&self) -> Option<String> {
        self.rtc_data.as_ref()?.sdp_answer.as_ref().map(|cb| cb.decompress())
    }
}