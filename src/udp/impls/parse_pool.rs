use crate::config::enums::cluster_mode::ClusterMode;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::enums::simple_proxy_protocol::SppParseResult;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::parse_spp_header;
use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::websocket::forward_request;
use crossbeam::queue::ArrayQueue;
use log::{
    debug,
    info,
    warn
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

const BATCH_SIZE: usize = 64;

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

    pub async fn start_thread(&self, threads: usize, tracker: Arc<TorrentTracker>, shutdown_handler: tokio::sync::watch::Receiver<bool>, use_payload_ip: bool, simple_proxy_protocol: bool) {
        let is_slave_mode = tracker.config.tracker_config.cluster == ClusterMode::slave;
        for i in 0..threads {
            let payload = self.payload.clone();
            let tracker_cloned = tracker.clone();
            let mut shutdown_handler = shutdown_handler.clone();
            let runtime = self.udp_runtime.clone();
            runtime.spawn(async move {
                info!("[UDP] Start Parse Pool thread {i}...");
                let mut batch: Vec<UdpPacket> = Vec::with_capacity(BATCH_SIZE);
                let mut interval = tokio::time::interval(Duration::from_micros(100));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        biased;
                        _ = shutdown_handler.changed() => {
                            info!("[UDP] Shutting down the Parse Pool thread {i}...");
                            return;
                        }
                        _ = interval.tick() => {
                            while batch.len() < BATCH_SIZE {
                                if let Some(packet) = payload.pop() {
                                    batch.push(packet);
                                } else {
                                    break;
                                }
                            }
                            if !batch.is_empty() {
                                for packet in batch.drain(..) {
                                    if is_slave_mode {
                                        Self::handle_slave_forward(
                                            &tracker_cloned,
                                            packet,
                                            simple_proxy_protocol,
                                        ).await;
                                    } else {
                                        let (effective_addr, payload_slice) = if simple_proxy_protocol {
                                            Self::extract_spp_info(&packet)
                                        } else {
                                            (packet.remote_addr, &packet.data[..packet.data_len])
                                        };
                                        let response = UdpServer::handle_packet(
                                            effective_addr,
                                            payload_slice,
                                            tracker_cloned.clone(),
                                            use_payload_ip
                                        ).await;
                                        UdpServer::send_response(
                                            tracker_cloned.clone(),
                                            packet.socket.clone(),
                                            packet.remote_addr,
                                            response
                                        ).await;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        let runtime = self.udp_runtime.clone();
        std::mem::forget(runtime);
    }

    fn extract_spp_info(packet: &UdpPacket) -> (SocketAddr, &[u8]) {
        let data = &packet.data[..packet.data_len];
        match parse_spp_header(data) {
            SppParseResult::Found { header, payload_offset } => {
                debug!(
                    "[UDP SPP] Extracted real client address: {} (proxy: {})",
                    header.client_socket_addr(),
                    header.proxy_socket_addr()
                );
                (header.client_socket_addr(), &data[payload_offset..])
            }
            SppParseResult::NotPresent => {
                (packet.remote_addr, data)
            }
            SppParseResult::Malformed(msg) => {
                warn!("[UDP SPP] Malformed SPP header: {}", msg);
                (packet.remote_addr, data)
            }
        }
    }

    async fn handle_slave_forward(tracker: &Arc<TorrentTracker>, packet: UdpPacket, simple_proxy_protocol: bool) {
        let (effective_addr, payload_data) = if simple_proxy_protocol {
            let (addr, slice) = Self::extract_spp_info(&packet);
            (addr, slice.to_vec())
        } else {
            (packet.remote_addr, packet.data[..packet.data_len].to_vec())
        };
        match forward_request(
            tracker,
            ProtocolType::Udp,
            RequestType::UdpPacket,
            effective_addr.ip(),
            effective_addr.port(),
            payload_data,
        ).await {
            Ok(response) => {
                UdpServer::send_packet(packet.socket, &packet.remote_addr, &response.payload).await;
            }
            Err(e) => {
                debug!("[UDP SLAVE] Failed to forward packet to master: {}", e);
            }
        }
    }
}