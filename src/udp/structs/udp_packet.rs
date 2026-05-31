use smallvec::SmallVec;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

pub const INLINE_PACKET_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct UdpPacket {
    pub remote_addr: SocketAddr,
    pub data: SmallVec<[u8; INLINE_PACKET_SIZE]>,
    pub socket: Arc<UdpSocket>,
}