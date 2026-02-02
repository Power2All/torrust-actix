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
    #[serde(default)]
    pub use_payload_ip: bool,
    /// Enable Simple Proxy Protocol (SPP) support for UDP.
    /// When enabled, the server will check for a 38-byte SPP header at the start of each packet.
    /// If present, the real client IP and port will be extracted from the header.
    /// This is used by Cloudflare Spectrum and other proxies that support SPP.
    #[serde(default)]
    pub simple_proxy_protocol: bool,
}