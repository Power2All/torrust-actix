use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use log::{error, info};
use tokio::task::JoinHandle;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::udp_server::UdpServer;

pub const PROTOCOL_IDENTIFIER: i64 = 4_497_486_125_440;
pub const MAX_SCRAPE_TORRENTS: u8 = 74;
pub const MAX_PACKET_SIZE: usize = 1496;

pub async fn udp_service(addr: SocketAddr, data: Arc<TorrentTracker>, rx: tokio::sync::watch::Receiver<bool>) -> JoinHandle<()>
{
    let udp_server = UdpServer::new(data, addr).await.unwrap_or_else(|e| {
        error!("Could not listen to the UDP port: {}", e);
        exit(1);
    });

    info!("[UDP] Starting server listener on {}", addr);
    tokio::spawn(async move {
        udp_server.start(rx).await;
    })
}
