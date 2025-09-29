use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use log::{info, warn};
use tokio::sync::mpsc;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::structs::udp_server::UdpServer;

impl Default for ParsePool {
    fn default() -> Self {
        Self::new(100, 10000, 5000)
    }
}

impl ParsePool {
    /// Creates a new ParsePool with dynamic capacity
    /// Uses an unbounded channel to prevent packet drops during burst traffic
    pub fn new(max_burst: usize, queue_threshold: usize, low_threshold: usize) -> ParsePool {
        let (tx, rx) = mpsc::unbounded_channel();
        ParsePool {
            sender: Arc::new(tx),
            receiver: Arc::new(tokio::sync::Mutex::new(rx)),
            queue_len: Arc::new(AtomicUsize::new(0)),
            max_burst,
            queue_threshold,
            low_threshold
        }
    }

    /// Starts worker threads that dynamically scale based on queue depth
    ///
    /// # Arguments
    /// * `base_threads` - Minimum number of worker threads to maintain
    /// * `tracker` - Shared tracker instance
    /// * `shutdown_handler` - Channel to receive shutdown signals
    pub async fn start_thread(
        &self,
        base_threads: usize,
        tracker: Arc<TorrentTracker>,
        shutdown_handler: tokio::sync::watch::Receiver<bool>
    ) {
        // Start base worker threads (always running)
        for i in 0..base_threads {
            let receiver = self.receiver.clone();
            let tracker_cloned = tracker.clone();
            let queue_len = self.queue_len.clone();
            let mut shutdown_handler = shutdown_handler.clone();

            tokio::spawn(async move {
                info!("[UDP] Starting base Parse Pool worker thread {i}...");

                loop {
                    tokio::select! {
                        _ = shutdown_handler.changed() => {
                            info!("[UDP] Shutting down Parse Pool worker thread {i}...");
                            return;
                        }
                        // Process packets one at a time for base threads
                        result = Self::receive_packet(receiver.clone(), queue_len.clone()) => {
                            if let Some(packet) = result {
                                Self::process_packet(packet, tracker_cloned.clone()).await;
                            }
                        }
                    }
                }
            });
        }

        // Start dynamic scaling monitor thread
        let receiver_monitor = self.receiver.clone();
        let tracker_monitor = tracker.clone();
        let queue_len_monitor = self.queue_len.clone();
        let max_burst_monitor = self.max_burst.clone();
        let queue_threshold_monitor = self.queue_threshold.clone();
        let low_threshold_monitor = self.low_threshold.clone();
        let mut shutdown_monitor = shutdown_handler.clone();

        tokio::spawn(async move {
            info!("[UDP] Starting dynamic worker scaling monitor...");
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            let mut burst_workers: Vec<tokio::task::JoinHandle<()>> = Vec::new();
            let max_burst_workers = max_burst_monitor; // Maximum number of burst workers
            let queue_threshold = queue_threshold_monitor; // Spawn burst worker if queue > this
            let low_threshold = low_threshold_monitor; // Keep burst workers if queue > this

            loop {
                tokio::select! {
                    _ = shutdown_monitor.changed() => {
                        info!("[UDP] Shutting down dynamic scaling monitor...");
                        // Cancel all burst workers
                        for handle in burst_workers {
                            handle.abort();
                        }
                        return;
                    }
                    _ = interval.tick() => {
                        // Get approximate queue length
                        let approx_queue_len = queue_len_monitor.load(Ordering::Relaxed);

                        // Clean up finished burst workers
                        burst_workers.retain(|handle| !handle.is_finished());

                        // Scale up: spawn burst workers if queue is large
                        if approx_queue_len > queue_threshold && burst_workers.len() < max_burst_workers {
                            let workers_to_spawn = (approx_queue_len / queue_threshold).clamp(1, 5);

                            for _ in 0..workers_to_spawn {
                                if burst_workers.len() >= max_burst_workers {
                                    break;
                                }

                                let receiver_burst = receiver_monitor.clone();
                                let tracker_burst = tracker_monitor.clone();
                                let queue_len_burst = queue_len_monitor.clone();
                                let worker_id = base_threads + burst_workers.len();

                                let handle = tokio::spawn(async move {
                                    info!("[UDP] Spawning burst worker {worker_id}");
                                    let mut batch = Vec::with_capacity(64);
                                    let timeout = Duration::from_millis(50);

                                    // Burst worker processes in batches and exits when idle
                                    loop {
                                        // Try to fill batch
                                        while batch.len() < 64 {
                                            match tokio::time::timeout(
                                                timeout,
                                                Self::receive_packet(receiver_burst.clone(), queue_len_burst.clone())
                                            ).await {
                                                Ok(Some(packet)) => batch.push(packet),
                                                Ok(None) | Err(_) => break,
                                            }
                                        }

                                        if !batch.is_empty() {
                                            Self::process_batch(batch, tracker_burst.clone()).await;
                                            batch = Vec::with_capacity(64);
                                        } else {
                                            // No work for timeout period, exit burst worker
                                            info!("[UDP] Burst worker {worker_id} idle, exiting");
                                            return;
                                        }
                                    }
                                });

                                burst_workers.push(handle);
                            }

                            warn!("[UDP] Queue depth: {}, Active burst workers: {}/{}",
                                  approx_queue_len, burst_workers.len(), max_burst_workers);
                        }

                        // Scale down happens automatically as burst workers exit when idle
                        if approx_queue_len < low_threshold && !burst_workers.is_empty() {
                            info!("[UDP] Queue normalized ({}), {} burst workers will exit when idle",
                                  approx_queue_len, burst_workers.len());
                        }
                    }
                }
            }
        });
    }

    /// Receives a single packet from the queue and decrements counter
    async fn receive_packet(
        receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<UdpPacket>>>,
        queue_len: Arc<AtomicUsize>
    ) -> Option<UdpPacket> {
        let mut rx = receiver.lock().await;
        let packet = rx.recv().await;
        if packet.is_some() {
            queue_len.fetch_sub(1, Ordering::Relaxed);
        }
        packet
    }

    /// Processes a single UDP packet
    async fn process_packet(packet: UdpPacket, tracker: Arc<TorrentTracker>) {
        let data_slice = &packet.data[..packet.data_len];
        let response = UdpServer::handle_packet(packet.remote_addr, data_slice, tracker.clone()).await;
        UdpServer::send_response(tracker, packet.socket, packet.remote_addr, response).await;
    }

    /// Processes a batch of UDP packets (used by burst workers)
    async fn process_batch(packets: Vec<UdpPacket>, tracker: Arc<TorrentTracker>) {
        for packet in packets {
            Self::process_packet(packet, tracker.clone()).await;
        }
    }

    /// Pushes a packet into the queue and increments counter
    /// Returns true on success, false if channel is closed
    pub fn push(&self, packet: UdpPacket) -> bool {
        match self.sender.send(packet) {
            Ok(_) => {
                self.queue_len.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(_) => false
        }
    }

    /// Returns approximate queue length
    pub fn len(&self) -> usize {
        self.queue_len.load(Ordering::Relaxed)
    }

    /// Returns true if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue_len.load(Ordering::Relaxed) == 0
    }
}