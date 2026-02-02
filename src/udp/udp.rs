use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::udp_server::UdpServer;
use log::{error, info};
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

pub const PROTOCOL_IDENTIFIER: i64 = 4_497_486_125_440;
pub const MAX_SCRAPE_TORRENTS: u8 = 74;
pub const MAX_PACKET_SIZE: usize = 1496;

#[allow(clippy::too_many_arguments)]
pub async fn udp_service(addr: SocketAddr, udp_threads: usize, worker_threads: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool, use_payload_ip: bool, simple_proxy_protocol: bool, data: Arc<TorrentTracker>, rx: tokio::sync::watch::Receiver<bool>, tokio_udp: Arc<Runtime>) -> JoinHandle<()>
{
    let udp_server = UdpServer::new(data, addr, udp_threads, worker_threads, recv_buffer_size, send_buffer_size, reuse_address, use_payload_ip, simple_proxy_protocol).await.unwrap_or_else(|e| {
        error!("Could not listen to the UDP port: {e}");
        exit(1);
    });
    let spp_status = if simple_proxy_protocol { " with Simple Proxy Protocol enabled" } else { "" };
    info!("[UDP] Starting a server listener on {addr} with {udp_threads} UDP threads and {worker_threads} worker threads{spp_status}");
    tokio_udp.spawn(async move {
        udp_server.start(rx).await;
    })
}