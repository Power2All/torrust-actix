use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use log::{debug, info, warn};
use tokio::runtime::Builder;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::structs::dynamic_queue::DynamicQueue;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::structs::parse_pool_stats::ParsePoolStats;

impl Default for ParsePool {
    fn default() -> Self {
        Self::new(0)
    }
}

impl ParsePool {
    pub fn new(initial_capacity: usize) -> Self {
        // Configure the dynamic queue
        let segment_size = 10000;  // Each segment holds 10000 packets
        let max_segments = 100;    // Maximum 100 segments = 1,00,000 packets max

        ParsePool {
            payload: Arc::new(DynamicQueue::new(initial_capacity, segment_size, max_segments)),
            stats_high_water_mark: Arc::new(AtomicUsize::new(0)),
            stats_grow_count: Arc::new(AtomicUsize::new(0)),
            stats_drops: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create with custom configuration
    pub fn new_with_config(initial_capacity: usize, segment_size: usize, max_capacity: usize) -> Self {
        let max_segments = (max_capacity + segment_size - 1) / segment_size;

        ParsePool {
            payload: Arc::new(DynamicQueue::new(initial_capacity, segment_size, max_segments)),
            stats_high_water_mark: Arc::new(AtomicUsize::new(0)),
            stats_grow_count: Arc::new(AtomicUsize::new(0)),
            stats_drops: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Start worker threads with guaranteed thread isolation
    pub async fn start_thread(
        &self,
        worker_threads: usize,
        tracker: Arc<TorrentTracker>,
        rx: tokio::sync::watch::Receiver<bool>
    ) {
        // Start monitoring task
        self.start_monitoring(tracker.clone(), rx.clone()).await;

        // Start worker threads
        self.start_dedicated_workers(worker_threads, tracker, rx).await;
    }

    /// Monitor queue health and statistics
    async fn start_monitoring(&self, tracker: Arc<TorrentTracker>, mut rx: tokio::sync::watch::Receiver<bool>) {
        let payload = self.payload.clone();
        let high_water = self.stats_high_water_mark.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            let mut shrink_counter = 0;

            loop {
                tokio::select! {
                    _ = rx.changed() => {
                        break;
                    }
                    _ = interval.tick() => {
                        let len = payload.len();
                        let capacity = payload.capacity();
                        let segments = payload.segments_count();

                        // Update high water mark
                        let mut current_high = high_water.load(Ordering::Relaxed);
                        while len > current_high {
                            match high_water.compare_exchange_weak(
                                current_high,
                                len,
                                Ordering::Release,
                                Ordering::Relaxed
                            ) {
                                Ok(_) => break,
                                Err(x) => current_high = x,
                            }
                        }

                        // Log statistics
                        tracker.set_stats(crate::stats::enums::stats_event::StatsEvent::UdpQueueLen, len as i64);

                        // Log if we're getting close to capacity
                        let usage_percent = (len * 100) / capacity.max(1);
                        if usage_percent > 80 {
                            warn!("Parse pool at {}% capacity ({}/{}, {} segments)",
                                  usage_percent, len, capacity, segments);
                        } else if usage_percent > 50 {
                            debug!("Parse pool at {}% capacity ({}/{}, {} segments)",
                                   usage_percent, len, capacity, segments);
                        }

                        // Periodically try to shrink if underutilized
                        shrink_counter += 1;
                        if shrink_counter >= 60 { // Every minute
                            payload.try_shrink();
                            shrink_counter = 0;
                        }
                    }
                }
            }

            info!("Parse pool monitor stopped. High water mark: {} packets",
                  high_water.load(Ordering::Relaxed));
        });
    }

    /// Dedicated worker threads using std::thread
    async fn start_dedicated_workers(
        &self,
        worker_threads: usize,
        tracker: Arc<TorrentTracker>,
        rx: tokio::sync::watch::Receiver<bool>
    ) {
        let mut thread_handles = Vec::with_capacity(worker_threads);

        for worker_idx in 0..worker_threads {
            let payload_clone = self.payload.clone();
            let tracker_clone = tracker.clone();
            let rx_clone = rx.clone();

            // Spawn a native OS thread with its own runtime
            let handle = std::thread::Builder::new()
                .name(format!("parse-worker-{}", worker_idx))
                .stack_size(2 * 1024 * 1024)  // 2MB stack
                .spawn(move || {
                    // Create a single-threaded runtime for this worker
                    let runtime = Builder::new_current_thread()
                        .thread_name(format!("parse-runtime-{}", worker_idx))
                        .enable_all()
                        .build()
                        .expect("Failed to create parse worker runtime");

                    runtime.block_on(async move {
                        info!("Parse worker {} started", worker_idx);
                        Self::adaptive_worker_loop(
                            payload_clone,
                            tracker_clone,
                            rx_clone,
                            worker_idx
                        ).await;
                        info!("Parse worker {} stopped", worker_idx);
                    });
                })
                .expect("Failed to spawn parse worker thread");

            thread_handles.push(handle);
        }

        // Monitor for shutdown
        let mut shutdown_rx = rx.clone();
        tokio::spawn(async move {
            shutdown_rx.changed().await.ok();
            info!("Signaling parse workers to shutdown");

            // Join all worker threads
            for (idx, handle) in thread_handles.into_iter().enumerate() {
                if let Err(e) = handle.join() {
                    warn!("Failed to join parse worker {}: {:?}", idx, e);
                }
            }
            info!("All parse workers shut down");
        });
    }

    /// Adaptive worker loop that adjusts processing rate based on queue depth
    async fn adaptive_worker_loop(
        payload: Arc<DynamicQueue>,
        tracker: Arc<TorrentTracker>,
        rx: tokio::sync::watch::Receiver<bool>,
        worker_idx: usize
    ) {
        let mut consecutive_empty = 0;
        let mut batch = Vec::with_capacity(32);

        loop {
            // Check for shutdown
            if rx.has_changed().unwrap_or(false) {
                if *rx.borrow() {
                    break;
                }
            }

            // Adaptive processing based on queue depth
            let queue_len = payload.len();
            let capacity = payload.capacity();
            let usage_percent = (queue_len * 100) / capacity.max(1);

            // Determine batch size based on queue pressure
            let batch_size = if usage_percent > 75 {
                32  // High pressure - process in large batches
            } else if usage_percent > 50 {
                16  // Medium pressure
            } else if usage_percent > 25 {
                8   // Low pressure
            } else {
                4   // Very low pressure - minimize latency
            };

            // Collect batch
            batch.clear();
            for _ in 0..batch_size {
                if let Some(packet) = payload.pop() {
                    batch.push(packet);
                } else {
                    break;
                }
            }

            if batch.is_empty() {
                consecutive_empty += 1;

                // Adaptive sleep based on how long queue has been empty
                let sleep_ms = match consecutive_empty {
                    0..=10 => 1,      // 1ms for first 10 empty polls
                    11..=100 => 5,    // 5ms for next 90
                    101..=1000 => 10, // 10ms for next 900
                    _ => 50,          // 50ms after that
                };

                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                continue;
            }

            consecutive_empty = 0;

            // Process batch
            let process_futures: Vec<_> = batch.drain(..).map(|packet| {
                let tracker_clone = tracker.clone();
                let socket_clone = packet.socket.clone();

                async move {
                    let response = UdpServer::handle_packet(
                        packet.remote_addr,
                        &packet.data[..packet.data_len],
                        tracker_clone.clone()
                    ).await;

                    UdpServer::send_response(
                        tracker_clone,
                        socket_clone,
                        packet.remote_addr,
                        response
                    ).await;
                }
            }).collect();

            // Process all packets in batch concurrently
            futures::future::join_all(process_futures).await;

            // Yield occasionally to prevent starving other tasks
            if worker_idx % 4 == 0 {
                tokio::task::yield_now().await;
            }
        }
    }

    /// Get current statistics
    pub fn get_stats(&self) -> ParsePoolStats {
        ParsePoolStats {
            current_size: self.payload.len(),
            current_capacity: self.payload.capacity(),
            segments: self.payload.segments_count(),
            high_water_mark: self.stats_high_water_mark.load(Ordering::Relaxed),
            grow_count: self.stats_grow_count.load(Ordering::Relaxed),
            drops: self.stats_drops.load(Ordering::Relaxed),
        }
    }
}