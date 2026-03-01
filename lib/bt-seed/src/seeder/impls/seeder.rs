use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::seeder::{
    fmt_bytes,
    generate_peer_id,
    handle_peer,
    SharedRateLimiter
};
use crate::seeder::structs::seeder::Seeder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::tracker::structs::tracker_client::TrackerClient;
use governor::Quota;
use governor::RateLimiter;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{
    AtomicU64,
    AtomicUsize,
    Ordering
};
use std::sync::Arc;

impl Seeder {
    pub fn new(config: SeederConfig, torrent_info: TorrentInfo) -> Self {
        let peer_id = generate_peer_id();
        Self {
            config,
            torrent_info: Arc::new(torrent_info),
            uploaded: Arc::new(AtomicU64::new(0)),
            peer_count: Arc::new(AtomicUsize::new(0)),
            peer_id,
        }
    }

    #[allow(dead_code)]
    pub fn uploaded_bytes(&self) -> u64 {
        self.uploaded.load(Ordering::Relaxed)
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                eprintln!("[BT] Missing file: {}", file.path.display());
                missing = true;
            }
        }
        if missing {
            return Err("one or more data files are missing — cannot seed".into());
        }
        let listen_addr = format!("0.0.0.0:{}", self.config.listen_port);
        let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
        println!("Seeding… on {} (Ctrl+C to stop)\n", listen_addr);
        let rate_limiter: Option<SharedRateLimiter> =
            self.config.upload_limit.and_then(|kbs| {
                NonZeroU32::new(kbs as u32 * 1024).map(|quota_cells| {
                    Arc::new(RateLimiter::direct(Quota::per_second(quota_cells)))
                })
            });
        if self.config.upnp {
            let port = self.config.listen_port;
            tokio::spawn(async move {
                let local_ip = {
                    let s = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
                    if s.connect("8.8.8.8:80").is_err() {
                        return;
                    }
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
                            port,
                            local_addr,
                            0,
                            "bt-seed",
                        ).await {
                            Ok(()) => log::info!("[UPnP] Port {} mapped successfully", port),
                            Err(e) => log::warn!("[UPnP] Port mapping failed: {}", e),
                        }
                    }
                    Err(e) => log::warn!("[UPnP] Gateway discovery failed: {}", e),
                }
            });
        }
        let mut announce_interval_secs: u64 = 300;
        let tracker_opt: Option<TrackerClient> = self.try_announce_start(&mut announce_interval_secs).await;
        if self.config.show_stats {
            let uploaded_stats = Arc::clone(&self.uploaded);
            let peer_count_stats = Arc::clone(&self.peer_count);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    let up = uploaded_stats.load(Ordering::Relaxed);
                    let pc = peer_count_stats.load(Ordering::Relaxed);
                    let now = chrono::Local::now();
                    println!(
                        "[{}] peers: {}  uploaded: {}",
                        now.format("%H:%M:%S"), pc, fmt_bytes(up)
                    );
                }
            });
        }
        if let Some(ref tracker) = tracker_opt {
            let tracker_ann = tracker.clone();
            let uploaded_ann = Arc::clone(&self.uploaded);
            tokio::spawn(async move {
                let interval = std::time::Duration::from_secs(announce_interval_secs);
                loop {
                    tokio::time::sleep(interval).await;
                    let up = uploaded_ann.load(Ordering::Relaxed);
                    match tracker_ann.announce(up, "").await {
                        Ok(resp) => {
                            log::info!("[Tracker] Re-announced (interval={}s)", resp.interval);
                        }
                        Err(e) => {
                            log::warn!("[Tracker] Re-announce failed: {}", e);
                        }
                    }
                }
            });
        }
        let info_hash = self.torrent_info.info_hash;
        let peer_id = self.peer_id;
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let ti = Arc::clone(&self.torrent_info);
                            let up = Arc::clone(&self.uploaded);
                            let pc = Arc::clone(&self.peer_count);
                            let rl = rate_limiter.clone();
                            tokio::spawn(async move {
                                handle_peer(stream, addr, info_hash, peer_id, ti, up, pc, rl).await;
                            });
                        }
                        Err(e) => {
                            log::warn!("[BT] Accept error: {}", e);
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n[BT] Shutting down…");
                    if let Some(ref tracker) = tracker_opt {
                        let uploaded = self.uploaded.load(Ordering::Relaxed);
                        log::info!("[Tracker] Sending 'stopped' announcement…");
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            tracker.announce(uploaded, "stopped"),
                        ).await {
                            Ok(Ok(_))  => log::info!("[Tracker] Stopped announcement sent"),
                            Ok(Err(e)) => log::warn!("[Tracker] Stopped announce failed: {}", e),
                            Err(_)     => log::warn!("[Tracker] Stopped announce timed out"),
                        }
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    async fn try_announce_start(&self, interval_out: &mut u64) -> Option<TrackerClient> {
        let urls = &self.torrent_info.tracker_urls;
        if urls.is_empty() {
            log::info!("[Tracker] No tracker configured — seeding without announcing.");
            return None;
        }
        for url in urls {
            let tracker = TrackerClient::new(
                url.clone(),
                self.torrent_info.info_hash,
                self.peer_id,
                self.config.listen_port,
                self.config.proxy.as_ref(),
            );
            match tracker.announce(0, "started").await {
                Ok(resp) => {
                    *interval_out = resp.interval.max(30);
                    log::info!("[Tracker] Announced to {}: interval={}s", url, interval_out);
                    return Some(tracker);
                }
                Err(e) => {
                    log::warn!("[Tracker] {} failed: {} — trying next", url, e);
                }
            }
        }
        log::warn!("[Tracker] All trackers failed — seeding without announcing.");
        None
    }
}