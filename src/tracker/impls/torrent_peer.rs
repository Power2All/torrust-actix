use crate::tracker::structs::torrent_peer::TorrentPeer;
use std::net::{
    IpAddr,
    SocketAddr
};

impl TorrentPeer {
    pub fn peer_addr_from_ip_and_port_and_opt_host_ip(remote_ip: IpAddr, port: u16) -> SocketAddr {
        SocketAddr::new(remote_ip, port)
    }
}