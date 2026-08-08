use crate::config::enums::cluster_mode::ClusterMode;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::enums::simple_proxy_protocol::SppParseResult;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::structs::udp_server::UdpServer;
use crate::udp::udp::{
    has_spp_magic,
    parse_spp_header
};
use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::websocket::forward_request;
use crossbeam_queue::ArrayQueue;
use log::{
    debug,
    info,
    warn
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

const BATCH_SIZE: usize = 64;
const POLL_MIN: Duration = Duration::from_micros(100);
const POLL_MAX: Duration = Duration::from_millis(1);

impl Default for ParsePool {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl ParsePool {
    /// Creates the bounded packet queue shared between receive backends and parse workers.
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

    /// Spawns `threads` async workers that drain the packet queue, run the UDP request pipeline
    /// and send replies, until the shutdown watch channel fires.
    pub async fn start_thread(&self, threads: usize, tracker: Arc<TorrentTracker>, shutdown_handler: tokio::sync::watch::Receiver<bool>, use_payload_ip: bool, simple_proxy_protocol: bool, proxy_addrs: Arc<Vec<std::net::IpAddr>>) {
        let is_slave_mode = tracker.config.tracker_config.cluster == ClusterMode::slave;
        for i in 0..threads {
            let payload = self.payload.clone();
            let tracker_cloned = tracker.clone();
            let mut shutdown_handler = shutdown_handler.clone();
            let runtime = self.udp_runtime.clone();
            let proxy_addrs = proxy_addrs.clone();
            runtime.spawn(async move {
                info!("[UDP] Start Parse Pool thread {i}...");
                let mut batch: Vec<UdpPacket> = Vec::with_capacity(BATCH_SIZE);
                let mut poll_interval = POLL_MIN;
                loop {
                    loop {
                        while batch.len() < BATCH_SIZE {
                            if let Some(packet) = payload.pop() {
                                batch.push(packet);
                            } else {
                                break;
                            }
                        }
                        if batch.is_empty() {
                            break;
                        }
                        poll_interval = POLL_MIN;
                        for packet in batch.drain(..) {
                            if is_slave_mode {
                                Self::handle_slave_forward(
                                    &tracker_cloned,
                                    packet,
                                    simple_proxy_protocol,
                                    &proxy_addrs,
                                ).await;
                            } else {
                                let (effective_addr, payload_slice) = if simple_proxy_protocol {
                                    Self::extract_spp_info(&packet, &proxy_addrs)
                                } else {
                                    (packet.remote_addr, packet.data.as_slice())
                                };
                                // The announce packet's IP field is unauthenticated, exactly like
                                // the SPP header, so it gets the same allowlist: honouring it from
                                // an arbitrary sender lets any client register a third party's
                                // address as a peer and point the swarm at it. An empty list keeps
                                // the legacy trust-anyone behaviour that config validation warns
                                // about at startup.
                                let payload_ip_allowed = use_payload_ip
                                    && (proxy_addrs.is_empty() || proxy_addrs.contains(&packet.remote_addr.ip()));
                                let response = UdpServer::handle_packet(
                                    effective_addr,
                                    payload_slice,
                                    tracker_cloned.clone(),
                                    payload_ip_allowed
                                ).await;
                                UdpServer::send_response(
                                    tracker_cloned.clone(),
                                    packet.reply.clone(),
                                    packet.remote_addr,
                                    response
                                ).await;
                            }
                        }
                        if shutdown_handler.has_changed().unwrap_or(true) {
                            info!("[UDP] Shutting down the Parse Pool thread {i}...");
                            return;
                        }
                    }
                    tokio::select! {
                        biased;
                        _ = shutdown_handler.changed() => {
                            info!("[UDP] Shutting down the Parse Pool thread {i}...");
                            return;
                        }
                        _ = tokio::time::sleep(poll_interval) => {
                            poll_interval = (poll_interval * 2).min(POLL_MAX);
                        }
                    }
                }
            });
        }
    }

    /// Resolves the effective client address for a datagram carrying a Simple Proxy Protocol
    /// header.
    ///
    /// `proxy_addrs` lists the sources permitted to speak SPP; the header is unauthenticated, so
    /// honouring it from an arbitrary sender lets anyone choose their recorded address. An empty
    /// list trusts any sender, which configuration validation warns about at startup.
    fn extract_spp_info<'a>(packet: &'a UdpPacket, proxy_addrs: &[std::net::IpAddr]) -> (SocketAddr, &'a [u8]) {
        let data = packet.data.as_slice();
        if !proxy_addrs.is_empty() && !proxy_addrs.contains(&packet.remote_addr.ip()) {
            if has_spp_magic(data) {
                debug!(
                    "[UDP SPP] Ignoring proxy header from untrusted source {}",
                    packet.remote_addr
                );
            }
            return (packet.remote_addr, data);
        }
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
                warn!("[UDP SPP] Malformed SPP header: {msg}");
                (packet.remote_addr, data)
            }
        }
    }

    async fn handle_slave_forward(tracker: &Arc<TorrentTracker>, packet: UdpPacket, simple_proxy_protocol: bool, proxy_addrs: &[std::net::IpAddr]) {
        let (effective_addr, payload_data) = if simple_proxy_protocol {
            let (addr, slice) = Self::extract_spp_info(&packet, proxy_addrs);
            (addr, slice.to_vec())
        } else {
            (packet.remote_addr, packet.data.to_vec())
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
                UdpServer::send_packet(packet.reply, &packet.remote_addr, &response.payload).await;
            }
            Err(e) => {
                debug!("[UDP SLAVE] Failed to forward packet to master: {e}");
            }
        }
    }
}