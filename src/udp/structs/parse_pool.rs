use std::sync::Arc;
use crate::udp::structs::udp_packet::UdpPacket;
use crossbeam::queue::ArrayQueue;

pub struct ParsePool {
    pub payload: Arc<ArrayQueue<UdpPacket>>,
    pub(crate) udp_runtime: tokio::runtime::Runtime,
}