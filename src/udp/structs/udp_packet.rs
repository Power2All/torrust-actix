use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::udp::udp::MAX_PACKET_SIZE;

#[derive(Debug, Clone)]
pub struct UdpPacket {
    pub remote_addr: SocketAddr,
    pub data: [u8; MAX_PACKET_SIZE],
    pub data_len: usize,
    pub socket: Arc<UdpSocket>,
}