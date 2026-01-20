use std::sync::Arc;
use std::time::Duration;
use log::info;
use crossbeam::queue::ArrayQueue;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::structs::udp_server::UdpServer;

impl Default for ParsePool {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl ParsePool {
    pub fn new(capacity: usize, threads: usize) -> ParsePool {
        let tokio_udp = tokio::runtime::Builder::new_multi_thread()
            .thread_name("worker")
            .worker_threads(threads)
            .enable_all()
            .build()
            .unwrap();

        ParsePool {
            payload: Arc::new(ArrayQueue::new(capacity)),
            udp_runtime: Arc::new(tokio_udp),
        }
    }

    pub async fn start_thread(&self, threads: usize, tracker: Arc<TorrentTracker>, shutdown_handler: tokio::sync::watch::Receiver<bool>) {
        for i in 0..threads {
            let payload = self.payload.clone();
            let tracker_cloned = tracker.clone();
            let mut shutdown_handler = shutdown_handler.clone();
            let runtime = self.udp_runtime.clone();

            runtime.spawn(async move {
                info!("[UDP] Start Parse Pool thread {i}...");
                let mut batch = Vec::with_capacity(32);
                let mut interval = tokio::time::interval(Duration::from_millis(1));

                loop {
                    tokio::select! {
                        _ = shutdown_handler.changed() => {
                            info!("[UDP] Shutting down the Parse Pool thread {i}...");
                            return;
                        }
                        _ = interval.tick() => {
                            
                            while let Some(packet) = payload.pop() {
                                batch.push(packet);
                                if batch.len() >= 32 { break; }
                            }

                            if !batch.is_empty() {
                                Self::process_batch(batch, tracker_cloned.clone()).await;
                                batch = Vec::with_capacity(32);
                            }
                        }
                    }
                }
            });
        }

        
        let runtime = self.udp_runtime.clone();
        std::mem::forget(runtime);
    }

    async fn process_batch(packets: Vec<UdpPacket>, tracker: Arc<TorrentTracker>) {
        for packet in packets {
            let response = UdpServer::handle_packet(packet.remote_addr, &packet.data[..packet.data_len], tracker.clone()).await;
            UdpServer::send_response(tracker.clone(), packet.socket.clone(), packet.remote_addr, response).await;
        }
    }
}