use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::seeder::{
    fmt_bytes,
    generate_peer_id,
    handle_peer
};
use crate::seeder::structs::seeder::Seeder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::tracker::structs::tracker_client::TrackerClient;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    AtomicUsize,
    Ordering
};

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
        let tracker = TrackerClient::new(
            self.config.tracker_url.clone(),
            self.torrent_info.info_hash,
            self.peer_id,
            self.config.listen_port,
        );
        let mut announce_interval_secs: u64 = 300;
        match tracker.announce(0, "started").await {
            Ok(resp) => {
                announce_interval_secs = resp.interval.max(30);
                log::info!("[Tracker] Announced: interval={}s", announce_interval_secs);
            }
            Err(e) => {
                log::warn!("[Tracker] Initial announce failed: {}", e);
            }
        }
        let listen_addr = format!("0.0.0.0:{}", self.config.listen_port);
        let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
        println!("Seeding… on {} (Ctrl+C to stop)\n", listen_addr);
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
        let info_hash = self.torrent_info.info_hash;
        let peer_id = self.peer_id;
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let ti = Arc::clone(&self.torrent_info);
                    let up = Arc::clone(&self.uploaded);
                    let pc = Arc::clone(&self.peer_count);
                    tokio::spawn(async move {
                        handle_peer(stream, addr, info_hash, peer_id, ti, up, pc).await;
                    });
                }
                Err(e) => {
                    log::warn!("[BT] Accept error: {}", e);
                }
            }
        }
    }
}