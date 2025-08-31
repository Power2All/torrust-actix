use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

#[derive(Debug)]
pub struct UdpServer {
    pub(crate) socket: Arc<UdpSocket>,
    pub(crate) tracker: Arc<TorrentTracker>,
    pub(crate) receiver_threads: u64,
    pub(crate) worker_threads: u64,
    pub(crate) queue_size: u64
}