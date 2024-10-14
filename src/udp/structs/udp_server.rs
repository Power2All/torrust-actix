use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub struct UdpServer {
    pub(crate) socket: Arc<UdpSocket>,
    pub(crate) threads: u64,
    pub(crate) tracker: Arc<TorrentTracker>,
}