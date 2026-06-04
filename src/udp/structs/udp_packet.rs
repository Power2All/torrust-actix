use crate::udp::enums::udp_reply::UdpReply;
use smallvec::SmallVec;
use std::net::SocketAddr;

pub const INLINE_PACKET_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct UdpPacket {
    pub remote_addr: SocketAddr,
    pub data: SmallVec<[u8; INLINE_PACKET_SIZE]>,
    pub reply: UdpReply,
}