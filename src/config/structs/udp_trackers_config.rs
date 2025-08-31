use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UdpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub threads: u64,
    pub receive_buffer_size: usize,
    pub send_buffer_size: usize,
    pub reuse_address: bool
}