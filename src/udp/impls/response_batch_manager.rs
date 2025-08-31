use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use std::collections::{HashMap, VecDeque};
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
            let mut buffer = VecDeque::with_capacity(1000);
            let mut timer = interval(Duration::from_millis(5));

            loop {
                tokio::select! {
                    Some(response) = receiver.recv() => {
                        buffer.push_back(response);

                        if buffer.len() >= 500 {
                            Self::flush_buffer(&socket, &mut buffer).await;
                        }
                    }

                    _ = timer.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_buffer(&socket, &mut buffer).await;
                        }
                    }
                }
            }
        });

        Self { sender }
    }

    pub(crate) fn queue_response(&self, remote_addr: SocketAddr, payload: Vec<u8>) {
        let _ = self.sender.send(QueuedResponse { remote_addr, payload });
    }

    async fn flush_buffer(socket: &UdpSocket, buffer: &mut VecDeque<QueuedResponse>) {
        while let Some(response) = buffer.pop_front() {
            let _ = socket.send_to(&response.payload, &response.remote_addr).await;
        }
    }

    pub(crate) async fn get_for_socket(socket: Arc<UdpSocket>) -> Arc<ResponseBatchManager> {
        let socket_key = format!("{:?}", socket.local_addr().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()));

        let registry = BATCH_MANAGERS.get_or_init(|| RwLock::new(HashMap::new()));

        if let Some(manager) = registry.read().await.get(&socket_key) {
            return manager.clone();
        }

        let manager = Arc::new(ResponseBatchManager::new(socket));
        registry.write().await.insert(socket_key, manager.clone());

        manager
    }
}