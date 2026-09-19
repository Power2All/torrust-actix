use crate::config::enums::udp_receive_method::UdpReceiveMethod;
use serde::{
    Deserialize,
    Serialize
};
use std::net::IpAddr;

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
    #[serde(default)]
    pub simple_proxy_protocol: bool,
    /// Source addresses allowed to prepend a Simple Proxy Protocol header. An empty list trusts
    /// the header from any sender, letting anyone choose the client address the tracker records.
    #[serde(default)]
    pub proxy_addresses: Vec<String>,
    /// Parsed form of [`Self::proxy_addresses`], filled in once at startup.
    #[serde(skip)]
    pub proxy_addrs: Vec<IpAddr>,
    #[serde(default)]
    pub receive_method: UdpReceiveMethod,
    /// Slots in the queue between the receive backend and the parse workers.
    ///
    /// Preallocated in full, at 312 bytes per slot. Raise it only if the receive backend is
    /// actually outrunning the workers in bursts; a deeper queue buys latency, not throughput.
    #[serde(default = "crate::config::config::default_parse_queue_size")]
    pub parse_queue_size: usize,
}