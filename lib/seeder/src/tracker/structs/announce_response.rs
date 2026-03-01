#[derive(Debug, Clone, Default)]
pub struct AnnounceResponse {
    pub interval: u64,
    pub peers: Vec<(std::net::Ipv4Addr, u16)>,
    pub failure_reason: Option<String>,
}