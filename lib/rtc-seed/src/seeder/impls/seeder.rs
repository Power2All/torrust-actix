use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::seeder::{
    fmt_bytes,
    generate_peer_id,
    SharedRateLimiter
};
use crate::seeder::structs::peer_conn::PeerConn;
use crate::seeder::structs::seeder::Seeder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::tracker::structs::http_client::TrackerClient;
use governor::Quota;
use governor::RateLimiter;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{
    AtomicU64,
    Ordering
};
use std::sync::Arc;

impl Seeder {
    pub fn new(config: SeederConfig, torrent_info: TorrentInfo) -> Self {
        let peer_id = generate_peer_id();
        Self {
            config,
            torrent_info: Arc::new(torrent_info),
            peers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            uploaded: Arc::new(AtomicU64::new(0)),
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
        // Print resolved data-file paths and validate they all exist.
        println!("Data  :");
        for file in &self.torrent_info.files {
            println!("  {}", file.path.display());
        }
        println!();
        let mut missing = false;
        for file in &self.torrent_info.files {
            if !file.path.exists() {
                eprintln!("[RTC] Missing file: {}", file.path.display());
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
        print!("Creating WebRTC offer (gathering ICE candidates)… ");
        let mut current_pc = PeerConn::new(
            &self.config,
            Arc::clone(&self.torrent_info),
            Arc::clone(&self.uploaded),
            rate_limiter.clone(),
        )
        .await?;
        println!("done.");
        println!("Seeding… (Ctrl+C to stop)\n");
        if self.config.show_stats {
            let peers_clone = Arc::clone(&self.peers);
            let uploaded_clone = Arc::clone(&self.uploaded);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    let peers = peers_clone.lock().await;
                    let peer_count = peers.len();
                    drop(peers);
                    let up = uploaded_clone.load(Ordering::Relaxed);
                    let now = chrono::Local::now();
                    println!(
                        "[{}] peers: {}  uploaded: {}",
                        now.format("%H:%M:%S"),
                        peer_count,
                        fmt_bytes(up)
                    );
                }
            });
        }
        let tracker_opt = self.pick_tracker().await;
        let mut event = "started";
        let mut rtc_interval_ms = self.config.rtc_interval_ms;
        loop {
            let uploaded = self.uploaded.load(Ordering::Relaxed);
            if let Some(tracker) = &tracker_opt {
                match tracker
                    .announce_seeder(&current_pc.sdp_offer, uploaded, event)
                    .await
                {
                    Ok(resp) => {
                        event = "";
                        if let Some(ri) = resp.rtc_interval {
                            rtc_interval_ms = ri * 1000;
                        }
                        for answer in resp.rtc_answers {
                            log::info!(
                                "[Seeder] Answer from peer {}…",
                                answer.peer_id_hex.get(..8).unwrap_or(&answer.peer_id_hex)
                            );
                            let next_pc = match PeerConn::new(
                                &self.config,
                                Arc::clone(&self.torrent_info),
                                Arc::clone(&self.uploaded),
                                rate_limiter.clone(),
                            )
                            .await
                            {
                                Ok(pc) => pc,
                                Err(e) => {
                                    log::error!("[Seeder] Failed to create new PeerConn: {}", e);
                                    break;
                                }
                            };
                            if let Err(e) = current_pc.handle_answer(answer.sdp_answer).await {
                                log::error!("[Seeder] handle_answer failed: {}", e);
                            }
                            {
                                let mut peers = self.peers.lock().await;
                                peers.insert(answer.peer_id_hex, Arc::new(current_pc));
                            }
                            current_pc = next_pc;
                        }
                    }
                    Err(e) => {
                        log::warn!("[Tracker] Announce failed: {}", e);
                    }
                }
            }
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(rtc_interval_ms)) => {}
                _ = tokio::signal::ctrl_c() => break,
            }
        }
        println!("\n[RTC] Shutting down…");
        if let Some(ref tracker) = tracker_opt {
            let uploaded = self.uploaded.load(Ordering::Relaxed);
            log::info!("[Tracker] Sending 'stopped' announcement…");
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tracker.announce_seeder("", uploaded, "stopped"),
            ).await {
                Ok(Ok(_))  => log::info!("[Tracker] Stopped announcement sent"),
                Ok(Err(e)) => log::warn!("[Tracker] Stopped announce failed: {}", e),
                Err(_)     => log::warn!("[Tracker] Stopped announce timed out"),
            }
        }
        Ok(())
    }

    async fn pick_tracker(&self) -> Option<TrackerClient> {
        let urls = &self.torrent_info.tracker_urls;
        if urls.is_empty() {
            log::info!("[Tracker] No tracker configured — seeding without announcing.");
            return None;
        }
        let url = &urls[0];
        log::info!("[Tracker] Using tracker: {}", url);
        Some(TrackerClient::new(
            url.clone(),
            self.torrent_info.info_hash,
            self.peer_id,
            self.config.proxy.as_ref(),
        ))
    }
}