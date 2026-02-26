use std::net::SocketAddr;

#[allow(dead_code)]
pub struct PeerConn {
    pub peer_id_hex: String,
    pub addr: SocketAddr,
}