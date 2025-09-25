use std::sync::Arc;
use log::info;
use crossbeam::queue::ArrayQueue;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::enums::request::Request;
use crate::udp::enums::response::Response;
use crate::udp::enums::server_error::ServerError;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::MAX_SCRAPE_TORRENTS;
use crate::stats::enums::stats_event::StatsEvent;

impl Default for ParsePool {
    fn default() -> Self {
        Self::new(0)
    }
}

impl ParsePool {
    pub fn new(capacity: usize) -> ParsePool {
        ParsePool { payload: Arc::new(ArrayQueue::new(capacity)) }
    }

    pub async fn start_thread(&self, threads: usize, tracker: Arc<TorrentTracker>, shutdown_handler: tokio::sync::watch::Receiver<bool>) {
        for i in 0..threads {
            let payload = self.payload.clone();
            let tracker_cloned = tracker.clone();
            let mut shutdown_handler = shutdown_handler.clone();

            tokio::spawn(async move {
                info!("[UDP] Start Parse Pool thread {i}...");
                let mut batch: Vec<UdpPacket> = Vec::with_capacity(64);

                // aggressive drain with periodic yields when empty
                const BATCH_MAX: usize = 64;
                const EMPTY_YIELD_EVERY: usize = 256;
                let mut empty_polls = 0usize;

                loop {
                    // Drain queue into the batch buffer
                    batch.clear();
                    while let Some(packet) = payload.pop() {
                        batch.push(packet);
                        if batch.len() >= BATCH_MAX {
                            break;
                        }
                    }

                    if !batch.is_empty() {
                        // Process without copying packet data
                        Self::process_batch(&batch, tracker_cloned.clone()).await;
                        empty_polls = 0;
                    } else {
                        empty_polls += 1;
                        if empty_polls % EMPTY_YIELD_EVERY == 0 {
                            tokio::task::yield_now().await;
                        }
                    }

                    // Low-cost shutdown check
                    if shutdown_handler.has_changed().unwrap_or(false)
                        && shutdown_handler.changed().await.is_ok() {
                            info!("[UDP] Shutting down the Parse Pool thread {i}...");
                            return;
                        }
                }
            });
        }
    }

    async fn process_batch(packets: &[UdpPacket], tracker: Arc<TorrentTracker>) {
        for packet in packets {
            let payload = &packet.data[..packet.data_len];

            let response = match Request::from_bytes(payload, MAX_SCRAPE_TORRENTS) {
                Ok(request) => {
                    match UdpServer::handle_request(request, packet.remote_addr, tracker.clone()).await {
                        Ok(resp) => resp,
                        Err(_e) => {
                            match packet.remote_addr {
                                std::net::SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4InvalidRequest, 1); }
                                std::net::SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6InvalidRequest, 1); }
                            }
                            Response::from(crate::udp::structs::error_response::ErrorResponse {
                                transaction_id: TransactionId(0),
                                message: ServerError::BadRequest.to_string().into(),
                            })
                        }
                    }
                }
                Err(_) => {
                    match packet.remote_addr {
                        std::net::SocketAddr::V4(_) => { tracker.update_stats(StatsEvent::Udp4BadRequest, 1); }
                        std::net::SocketAddr::V6(_) => { tracker.update_stats(StatsEvent::Udp6BadRequest, 1); }
                    }
                    Response::from(crate::udp::structs::error_response::ErrorResponse {
                        transaction_id: TransactionId(0),
                        message: ServerError::BadRequest.to_string().into(),
                    })
                }
            };

            UdpServer::send_response(tracker.clone(), packet.socket.clone(), packet.remote_addr, response).await;
        }
    }
}