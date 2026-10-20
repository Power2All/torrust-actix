use std::sync::Arc;
use crate::udp::structs::udp_packet::UdpPacket;
use crossbeam_queue::ArrayQueue;

pub struct ParsePool {
    pub payload: Arc<ArrayQueue<UdpPacket>>,
    /// Always `Some` until dropped; see the `Drop` impl for why it is an `Option`.
    pub udp_runtime: Option<tokio::runtime::Runtime>,
}