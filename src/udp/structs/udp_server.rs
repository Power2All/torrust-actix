use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

#[derive(Debug)]
pub struct UdpServer {
    pub(crate) socket: Arc<UdpSocket>,
    pub(crate) udp_threads: usize,
    pub(crate) worker_threads: usize,
    pub(crate) tracker: Arc<TorrentTracker>,
    pub(crate) use_payload_ip: bool,
    pub(crate) simple_proxy_protocol: bool,
}