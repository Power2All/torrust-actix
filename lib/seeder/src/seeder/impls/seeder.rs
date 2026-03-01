use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::seeder::{
    fmt_bytes,
    generate_peer_id,
    handle_peer,
    SharedRateLimiter
};
use crate::seeder::structs::peer_conn::PeerConn;
use crate::seeder::structs::seeder::Seeder;
use crate::seeder::structs::torrent_registry::{
    TorrentRegistry,
    TorrentRegistryEntry
};
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::tracker::structs::bt_client::BtTrackerClient;
use crate::tracker::structs::rtc_client::RtcTrackerClient;
use governor::Quota;
use governor::RateLimiter;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{
    AtomicU64,
    AtomicUsize,
    Ordering
};
use std::sync::Arc;
use tokio::sync::Mutex;

impl Seeder {
    pub fn new(config: SeederConfig, torrent_info: TorrentInfo) -> Self {
        let peer_id = generate_peer_id();
        Self {
            config,
            torrent_info: Arc::new(torrent_info),
            uploaded: Arc::new(AtomicU64::new(0)),
            peer_count: Arc::new(AtomicUsize::new(0)),
            peers: Arc::new(Mutex::new(HashMap::new())),
            peer_id,
        }
    }

    #[allow(dead_code)]
    pub fn uploaded_bytes(&self) -> u64 {
        self.uploaded.load(Ordering::Relaxed)
    }

    pub async fn run(
        &mut self,
        registry: Option<TorrentRegistry>,
        mut ext_stop_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let out_path = self.config.out_file.clone().unwrap_or_else(|| {
            PathBuf::from(format!("{}.torrent", self.torrent_info.name))
        });
        std::fs::write(&out_path, &self.torrent_info.torrent_bytes)?;
        println!("Saved : {}", out_path.display());
        println!("Hash  : {}", hex::encode(self.torrent_info.info_hash));
        if let Some(v2h) = self.torrent_info.v2_info_hash {
            println!("v2Hash: {}", hex::encode(v2h));
        }
        println!("\nMagnet URI:\n{}\n", self.torrent_info.magnet_uri);
        println!("Share the magnet URI or the .torrent file with leechers.\n");
        println!("Data  :");
        for file in &self.torrent_info.files {
            println!("  {}", file.path.display());
        }
        println!();
        let mut missing = false;
        for file in &self.torrent_info.files {
            if !file.path.exists() {
                eprintln!("[Seeder] Missing file: {}", file.path.display());
                missing = true;
            }
        }
        if missing {
            return Err("one or more data files are missing — cannot seed".into());
        }

        let rate_limiter: Option<SharedRateLimiter> =
            self.config.upload_limit.and_then(|kbs| {
                NonZeroU32::new(kbs as u32 * 1024).map(|quota_cells| {
                    Arc::new(RateLimiter::direct(Quota::per_second(quota_cells)))
                })
            });

        let protocol = &self.config.protocol;
        if self.config.show_stats {
            let uploaded_stats = Arc::clone(&self.uploaded);
            let peer_count_stats = Arc::clone(&self.peer_count);
            let peers_stats = Arc::clone(&self.peers);
            let has_bt = protocol.has_bt();
            let has_rtc = protocol.has_rtc();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    let up = uploaded_stats.load(Ordering::Relaxed);
                    let now = chrono::Local::now();
                    let mut parts = Vec::new();
                    if has_bt {
                        let pc = peer_count_stats.load(Ordering::Relaxed);
                        parts.push(format!("bt_peers: {}", pc));
                    }
                    if has_rtc {
                        let rtc_count = peers_stats.lock().await.len();
                        parts.push(format!("rtc_peers: {}", rtc_count));
                    }
                    parts.push(format!("uploaded: {}", fmt_bytes(up)));
                    println!("[{}] {}", now.format("%H:%M:%S"), parts.join("  "));
                }
            });
        }
        let mut bt_announce_interval: u64 = 300;
        let bt_tracker_opt: Option<BtTrackerClient> = if protocol.has_bt() {
            self.try_bt_announce_start(&mut bt_announce_interval).await
        } else {
            None
        };
        let mut rtc_tracker_opt: Option<RtcTrackerClient> = None;
        let mut initial_peer_conn: Option<PeerConn> = None;
        if protocol.has_rtc()
            && let Some(tc) = self.pick_rtc_tracker() {
                rtc_tracker_opt = Some(tc);
                print!("Creating WebRTC offer (gathering ICE candidates)… ");
                match PeerConn::new(
                    &self.config,
                    Arc::clone(&self.torrent_info),
                    Arc::clone(&self.uploaded),
                    rate_limiter.clone(),
                ).await {
                    Ok(pc) => {
                        println!("done.");
                        initial_peer_conn = Some(pc);
                    }
                    Err(e) => {
                        log::error!("[RTC] Failed to create initial PeerConn: {}", e);
                    }
                }
            }

        println!("Seeding… (Ctrl+C to stop)\n");
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let bt_reannounce_handle = if let Some(ref tracker) = bt_tracker_opt {
            let tracker_ann = tracker.clone();
            let uploaded_ann = Arc::clone(&self.uploaded);
            let mut srx = stop_rx.clone();
            Some(tokio::spawn(async move {
                let interval = std::time::Duration::from_secs(bt_announce_interval);
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(interval) => {
                            let up = uploaded_ann.load(Ordering::Relaxed);
                            match tracker_ann.announce(up, "").await {
                                Ok(resp) => {
                                    log::info!("[Tracker/BT] Re-announced (interval={}s)", resp.interval);
                                }
                                Err(e) => {
                                    log::warn!("[Tracker/BT] Re-announce failed: {}", e);
                                }
                            }
                        }
                        _ = srx.changed() => {
                            if *srx.borrow() { break; }
                        }
                    }
                }
            }))
        } else {
            None
        };
        if protocol.has_bt() {
            if let Some(ref reg) = registry {
                let entry = TorrentRegistryEntry {
                    torrent_info: Arc::clone(&self.torrent_info),
                    uploaded: Arc::clone(&self.uploaded),
                    peer_count: Arc::clone(&self.peer_count),
                    our_peer_id: self.peer_id,
                    rate_limiter: rate_limiter.clone(),
                };
                let mut map = reg.write().await;
                map.insert(self.torrent_info.info_hash, entry);
            } else {
                let listen_addr = format!("0.0.0.0:{}", self.config.listen_port);
                let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
                log::info!("[BT] Listening on {}", listen_addr);
                if self.config.upnp {
                    let port = self.config.listen_port;
                    tokio::spawn(async move {
                        let local_ip = {
                            let s = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
                            if s.connect("8.8.8.8:80").is_err() { return; }
                            match s.local_addr().unwrap().ip() {
                                std::net::IpAddr::V4(v4) => v4,
                                _ => return,
                            }
                        };
                        match igd_next::aio::tokio::search_gateway(Default::default()).await {
                            Ok(gw) => {
                                let local_addr = std::net::SocketAddr::V4(
                                    std::net::SocketAddrV4::new(local_ip, port)
                                );
                                match gw.add_port(
                                    igd_next::PortMappingProtocol::TCP,
                                    port, local_addr, 0, "seeder",
                                ).await {
                                    Ok(()) => log::info!("[UPnP] Port {} mapped successfully", port),
                                    Err(e) => log::warn!("[UPnP] Port mapping failed: {}", e),
                                }
                            }
                            Err(e) => log::warn!("[UPnP] Gateway discovery failed: {}", e),
                        }
                    });
                }
                let info_hash = self.torrent_info.info_hash;
                let peer_id = self.peer_id;
                let uploaded_bt = Arc::clone(&self.uploaded);
                let peer_count_bt = Arc::clone(&self.peer_count);
                let torrent_info_bt = Arc::clone(&self.torrent_info);
                let rl_bt = rate_limiter.clone();
                let mut srx = stop_rx.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            result = listener.accept() => {
                                match result {
                                    Ok((stream, addr)) => {
                                        let ti = Arc::clone(&torrent_info_bt);
                                        let up = Arc::clone(&uploaded_bt);
                                        let pc = Arc::clone(&peer_count_bt);
                                        let rl = rl_bt.clone();
                                        tokio::spawn(async move {
                                            handle_peer(stream, addr, info_hash, peer_id, ti, up, pc, rl).await;
                                        });
                                    }
                                    Err(e) => {
                                        log::warn!("[BT] Accept error: {}", e);
                                    }
                                }
                            }
                            _ = srx.changed() => {
                                if *srx.borrow() { break; }
                            }
                        }
                    }
                });
            }
        }
        let rtc_handle = if protocol.has_rtc()
            && let Some(tracker) = rtc_tracker_opt.clone()
            && let Some(current_pc) = initial_peer_conn.take()
        {
            let mut current_pc = current_pc;
            let peers = Arc::clone(&self.peers);
            let uploaded_rtc = Arc::clone(&self.uploaded);
            let config_rtc = self.config.clone();
            let torrent_info_rtc = Arc::clone(&self.torrent_info);
            let rl_rtc = rate_limiter.clone();
            let mut srx = stop_rx.clone();
            Some(tokio::spawn(async move {
                let mut event = "started";
                let mut rtc_interval_ms = config_rtc.rtc_interval_ms;
                loop {
                    let uploaded = uploaded_rtc.load(Ordering::Relaxed);
                    match tracker.announce_seeder(&current_pc.sdp_offer, uploaded, event).await {
                        Ok(resp) => {
                            event = "";
                            if let Some(ri) = resp.rtc_interval {
                                rtc_interval_ms = ri * 1000;
                            }
                            for answer in resp.rtc_answers {
                                log::info!(
                                    "[RTC] Answer from peer {}…",
                                    answer.peer_id_hex.get(..8).unwrap_or(&answer.peer_id_hex)
                                );
                                let next_pc = match PeerConn::new(
                                    &config_rtc,
                                    Arc::clone(&torrent_info_rtc),
                                    Arc::clone(&uploaded_rtc),
                                    rl_rtc.clone(),
                                ).await {
                                    Ok(pc) => pc,
                                    Err(e) => {
                                        log::error!("[RTC] Failed to create new PeerConn: {}", e);
                                        break;
                                    }
                                };
                                if let Err(e) = current_pc.handle_answer(answer.sdp_answer).await {
                                    log::error!("[RTC] handle_answer failed: {}", e);
                                }
                                {
                                    let mut p = peers.lock().await;
                                    p.insert(answer.peer_id_hex, Arc::new(current_pc));
                                }
                                current_pc = next_pc;
                            }
                        }
                        Err(e) => {
                            log::warn!("[Tracker/RTC] Announce failed: {}", e);
                        }
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_millis(rtc_interval_ms)) => {}
                        _ = srx.changed() => {
                            if *srx.borrow() {
                                let up = uploaded_rtc.load(Ordering::Relaxed);
                                let _ = tokio::time::timeout(
                                    std::time::Duration::from_secs(5),
                                    tracker.announce_seeder("", up, "stopped"),
                                ).await;
                                break;
                            }
                        }
                    }
                }
            }))
        } else {
            None
        };
        // Wait for either Ctrl+C or an external stop signal (e.g. torrent disabled/deleted via UI).
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n[Seeder] Ctrl+C received — shutting down…");
            }
            _ = async {
                loop {
                    ext_stop_rx.changed().await.ok();
                    if *ext_stop_rx.borrow() { break; }
                }
            } => {
                log::info!("[Seeder] Stop signal received — shutting down…");
            }
        }
        stop_tx.send(true).ok();
        if let Some(h) = bt_reannounce_handle { h.abort(); }
        if let Some(h) = rtc_handle {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(6), h).await;
        }
        if protocol.has_bt()
            && let Some(ref reg) = registry {
                let mut map = reg.write().await;
                map.remove(&self.torrent_info.info_hash);
            }
        if let Some(ref tracker) = bt_tracker_opt {
            let uploaded = self.uploaded.load(Ordering::Relaxed);
            log::info!("[Tracker/BT] Sending 'stopped' announcement…");
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tracker.announce(uploaded, "stopped"),
            ).await {
                Ok(Ok(_))  => log::info!("[Tracker/BT] Stopped announcement sent"),
                Ok(Err(e)) => log::warn!("[Tracker/BT] Stopped announce failed: {}", e),
                Err(_)     => log::warn!("[Tracker/BT] Stopped announce timed out"),
            }
        }

        Ok(())
    }

    async fn try_bt_announce_start(&self, interval_out: &mut u64) -> Option<BtTrackerClient> {
        let urls = &self.torrent_info.tracker_urls;
        if urls.is_empty() {
            log::info!("[Tracker/BT] No tracker configured — seeding without announcing.");
            return None;
        }
        for url in urls {
            if url.starts_with("udp://") || url.starts_with("http://") || url.starts_with("https://") {
                let tracker = BtTrackerClient::new(
                    url.clone(),
                    self.torrent_info.info_hash,
                    self.peer_id,
                    self.config.listen_port,
                    self.config.proxy.as_ref(),
                );
                match tracker.announce(0, "started").await {
                    Ok(resp) => {
                        *interval_out = resp.interval.max(30);
                        log::info!("[Tracker/BT] Announced to {}: interval={}s", url, interval_out);
                        return Some(tracker);
                    }
                    Err(e) => {
                        log::warn!("[Tracker/BT] {} failed: {} — trying next", url, e);
                    }
                }
            }
        }
        log::warn!("[Tracker/BT] All trackers failed — seeding without BT announcing.");
        None
    }

    fn pick_rtc_tracker(&self) -> Option<RtcTrackerClient> {
        let urls = &self.torrent_info.tracker_urls;
        if urls.is_empty() {
            log::info!("[Tracker/RTC] No tracker configured — seeding without announcing.");
            return None;
        }
        for url in urls {
            if url.starts_with("http://") || url.starts_with("https://") {
                log::info!("[Tracker/RTC] Using tracker: {}", url);
                return Some(RtcTrackerClient::new(
                    url.clone(),
                    self.torrent_info.info_hash,
                    self.peer_id,
                    self.config.proxy.as_ref(),
                ));
            }
        }
        log::warn!("[Tracker/RTC] No HTTP tracker found — RTC seeding without announcing.");
        None
    }
}