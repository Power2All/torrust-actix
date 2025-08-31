use std::net::SocketAddr;

pub struct PacketJob {
    pub(crate) data: Vec<u8>,
    pub(crate) remote_addr: SocketAddr,
}