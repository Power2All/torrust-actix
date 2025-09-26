use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use parking_lot::RwLock;
use crate::udp::structs::udp_packet::UdpPacket;

pub struct DynamicQueue {
    pub(crate) segments: Arc<RwLock<Vec<Arc<crossbeam::queue::ArrayQueue<UdpPacket>>>>>,
    pub(crate) current_write_segment: AtomicUsize,
    pub(crate) current_read_segment: AtomicUsize,
    pub(crate) segment_size: usize,
    pub(crate) max_segments: usize,
    pub(crate) total_capacity: AtomicUsize,
    pub(crate) total_items: AtomicUsize,
    pub(crate) is_growing: AtomicBool,
}