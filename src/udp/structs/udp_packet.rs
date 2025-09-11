use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

#[derive(Debug, Clone)]
pub struct UdpPacket {
    pub remote_addr: SocketAddr,
    pub data: Arc<[u8]>,
    pub socket: Arc<UdpSocket>,
}