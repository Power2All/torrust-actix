use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use std::collections::{HashMap, VecDeque};
use log::{debug, info};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use crate::udp::structs::queued_response::QueuedResponse;
use crate::udp::structs::response_batch_manager::ResponseBatchManager;

static BATCH_MANAGERS: OnceLock<RwLock<HashMap<String, Arc<ResponseBatchManager>>>> = OnceLock::new();

impl ResponseBatchManager {
    fn new(socket: Arc<UdpSocket>) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<QueuedResponse>();

        tokio::spawn(async move {
            let mut buffer = VecDeque::with_capacity(1000); // Larger buffer for higher throughput
            let mut timer = interval(Duration::from_millis(5)); // 5ms flush interval for better responsiveness
            let mut stats_timer = interval(Duration::from_secs(10)); // Stats every 10 seconds
            let mut total_queued = 0u64;

            loop {
                tokio::select! {
                    // Receive responses
                    Some(response) = receiver.recv() => {
                        total_queued += 1;
                        buffer.push_back(response);

                        // If buffer reaches 500 responses, send immediately
                        if buffer.len() >= 500 {
                            debug!("Buffer full ({} items) - flushing immediately", buffer.len());
                            Self::flush_buffer(&socket, &mut buffer).await;
                        }
                    }

                    // Timer tick - flush any remaining responses
                    _ = timer.tick() => {
                        if !buffer.is_empty() {
                            debug!("Timer flush - {} items in buffer", buffer.len());
                            Self::flush_buffer(&socket, &mut buffer).await;
                        }
                    }

                    // Stats reporting
                    _ = stats_timer.tick() => {
                        info!("Batch sender stats - Queued: {}, Current buffer: {}, Socket: {:?}",
                              total_queued, buffer.len(), socket.local_addr());
                    }
                }
            }
        });

        Self { sender }
    }

    pub(crate) fn queue_response(&self, remote_addr: SocketAddr, payload: Vec<u8>) {
        // Monitor queue health
        match self.sender.send(QueuedResponse { remote_addr, payload }) {
            Ok(_) => {
                debug!("Response queued for {}", remote_addr);
            }
            Err(e) => {
                // This indicates the batch sender task has died
                log::error!("Failed to queue response - batch sender may have crashed: {}", e);
            }
        }
    }

    async fn flush_buffer(socket: &UdpSocket, buffer: &mut VecDeque<QueuedResponse>) {
        let batch_size = buffer.len();
        let mut sent_count = 0;
        let mut error_count = 0;

        // Process all responses in buffer
        while let Some(response) = buffer.pop_front() {
            match socket.send_to(&response.payload, &response.remote_addr).await {
                Ok(bytes_sent) => {
                    sent_count += 1;
                    debug!("Sent {} bytes to {}", bytes_sent, response.remote_addr);
                }
                Err(e) => {
                    error_count += 1;
                    match e.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            debug!("Send buffer full (EWOULDBLOCK) - packet dropped");
                        }
                        std::io::ErrorKind::Other => {
                            if let Some(os_error) = e.raw_os_error() {
                                match os_error {
                                    105 => debug!("ENOBUFS: No buffer space available - increase socket buffers"),
                                    111 => debug!("ECONNREFUSED: Connection refused by peer"),
                                    113 => debug!("EHOSTUNREACH: Host unreachable"),
                                    _ => debug!("Send error (OS error {}): {}", os_error, e),
                                }
                            } else {
                                debug!("Send error: {}", e);
                            }
                        }
                        _ => debug!("Send error: {}", e),
                    }
                }
            }
        }

        if batch_size > 0 {
            info!("Batch flush: {} total, {} sent, {} errors", batch_size, sent_count, error_count);
        }
    }

    // Get or create batch manager for a socket
    pub(crate) async fn get_for_socket(socket: Arc<UdpSocket>) -> Arc<ResponseBatchManager> {
        let socket_key = format!("{:?}", socket.local_addr().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()));

        // Initialize global registry if needed
        let registry = BATCH_MANAGERS.get_or_init(|| RwLock::new(HashMap::new()));

        // Try to get existing batch manager
        if let Some(manager) = registry.read().await.get(&socket_key) {
            return manager.clone();
        }

        // Create new batch manager
        let manager = Arc::new(ResponseBatchManager::new(socket));
        registry.write().await.insert(socket_key, manager.clone());

        manager
    }
}