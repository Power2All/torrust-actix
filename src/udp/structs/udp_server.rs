use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::config::enums::udp_receive_method::UdpReceiveMethod;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

#[derive(Debug)]
pub struct UdpServer {
    pub(crate) sockets: Vec<Arc<UdpSocket>>,
    pub(crate) udp_threads: usize,
    pub(crate) worker_threads: usize,
    pub(crate) tracker: Arc<TorrentTracker>,
    pub(crate) use_payload_ip: bool,
    pub(crate) simple_proxy_protocol: bool,
    pub(crate) receive_method: UdpReceiveMethod,
}