use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use crate::udp::structs::dynamic_queue::DynamicQueue;

pub struct ParsePool {
    pub payload: Arc<DynamicQueue>,
    pub(crate) stats_high_water_mark: Arc<AtomicUsize>,
    pub(crate) stats_grow_count: Arc<AtomicUsize>,
    pub(crate) stats_drops: Arc<AtomicUsize>,
}