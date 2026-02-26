use crate::seeder::structs::peer_conn::PeerConn;
use std::net::SocketAddr;

impl PeerConn {
    #[allow(dead_code)]
    pub fn new(peer_id_hex: String, addr: SocketAddr) -> Self {
        Self { peer_id_hex, addr }
    }
}