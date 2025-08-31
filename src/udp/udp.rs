use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use log::{error, info};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::udp_server::UdpServer;

pub const PROTOCOL_IDENTIFIER: i64 = 4_497_486_125_440;
pub const MAX_SCRAPE_TORRENTS: u8 = 74;
pub const MAX_PACKET_SIZE: usize = 1496;

#[allow(clippy::too_many_arguments)]
pub async fn udp_service(
    addr: SocketAddr,
    recv_buffer_size: usize,
    send_buffer_size: usize,
    reuse_address: bool,
    receiver_threads: usize,
    worker_threads: usize,
    queue_size: usize,
    data: Arc<TorrentTracker>,
    rx: tokio::sync::watch::Receiver<bool>,
    tokio_udp: Arc<Runtime>
) -> JoinHandle<()>
{
    let udp_server = UdpServer::new(
        data,
        addr,
        recv_buffer_size,
        send_buffer_size,
        reuse_address,
        receiver_threads,
        worker_threads,
        queue_size
    ).await.unwrap_or_else(|e| {
        error!("Could not listen to the UDP port: {e}");
        exit(1);
    });

    info!("[UDP] Starting server listener on {addr}");
    tokio_udp.spawn(async move {
        udp_server.start(rx).await;
    })
}