use std::sync::Arc;
use parking_lot::RwLock;
use crate::udp::structs::udp_packet::UdpPacket;

pub struct ParsePool {
    pub payload: Arc<RwLock<Vec<UdpPacket>>>
}