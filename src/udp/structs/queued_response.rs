use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct QueuedResponse {
    pub(crate) remote_addr: SocketAddr,
    pub(crate) payload: Vec<u8>,
}