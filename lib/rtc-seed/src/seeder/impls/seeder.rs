use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::seeder::{
    fmt_bytes,
    generate_peer_id
};
use crate::seeder::structs::peer_conn::PeerConn;
use crate::seeder::structs::seeder::Seeder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::tracker::structs::http_client::TrackerClient;
use std::collections::HashMap;
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
        println!("\nMagnet URI:\n{}\n", self.torrent_info.magnet_uri);
        println!("Share the magnet URI or the .torrent file with leechers.\n");
        let tracker = TrackerClient::new(
            self.config.tracker_url.clone(),
            self.torrent_info.info_hash,
            self.peer_id,
        );
        print!("Creating WebRTC offer (gathering ICE candidates)… ");
        let mut current_pc = PeerConn::new(
            &self.config,
            Arc::clone(&self.torrent_info),
            Arc::clone(&self.uploaded),
        )
        .await?;
        println!("done.");
        println!("Seeding… (Ctrl+C to stop)\n");
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
        let mut event = "started";
        let mut rtc_interval_ms = self.config.rtc_interval_ms;
        loop {
            let uploaded = self.uploaded.load(Ordering::Relaxed);
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

            tokio::time::sleep(std::time::Duration::from_millis(rtc_interval_ms)).await;
        }
    }
}