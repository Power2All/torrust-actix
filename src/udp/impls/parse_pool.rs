use std::sync::Arc;
use log::{debug, info};
use parking_lot::RwLock;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_server::UdpServer;

impl Default for ParsePool {
    fn default() -> Self {
        Self::new()
    }
}

impl ParsePool {
    pub fn new() -> ParsePool
    {
        ParsePool { payload: Arc::new(RwLock::new(Vec::new())) }
    }

    pub async fn start_thread(&self, threads: usize, tracker: Arc<TorrentTracker>, shutdown_handler: tokio::sync::watch::Receiver<bool>) {
        for i in 0..threads {
            let payload = self.payload.clone();
            let tracker_cloned = tracker.clone();
            let mut shutdown_handler = shutdown_handler.clone();

            tokio::spawn(async move {
                info!("[UDP] Start Parse Pool thread {i}...");
                loop {
                    tokio::select! {
                        _ = shutdown_handler.changed() => {
                            info!("[UDP] Shutting down the Parse Pool thread {i}...");
                            return;
                        }
                        else => {
                            let batch_opt = {
                                let mut guard = payload.write();
                                if guard.is_empty() {
                                    None
                                } else {
                                    Some(std::mem::take(&mut *guard))
                                }
                            };
                            if let Some(data) = batch_opt {
                                for udp_packet in data {
                                    debug!("Executing request IP {}", udp_packet.remote_addr);
                                    let response = UdpServer::handle_packet(udp_packet.remote_addr, &udp_packet.data, tracker_cloned.clone()).await;
                                    UdpServer::send_response(tracker_cloned.clone(), udp_packet.socket.clone(), udp_packet.remote_addr, response).await;
                                }
                            } else {
                                tokio::task::yield_now().await;
                            }
                        }
                    }
                }
            });
        }
    }
}