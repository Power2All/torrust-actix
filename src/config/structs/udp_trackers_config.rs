use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UdpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub udp_threads: usize,
    pub worker_threads: usize,
    pub receive_buffer_size: usize,
    pub send_buffer_size: usize,
    pub reuse_address: bool,
    pub initial_capacity: usize,
    pub segment_size: usize,
    pub max_capacity: usize,
}