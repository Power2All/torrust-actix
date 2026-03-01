use crate::config::enums::seed_protocol::SeedProtocol;
use crate::config::structs::seeder_config::SeederConfig;
use crate::torrent::enums::torrent_version::TorrentVersion;

impl Default for SeederConfig {
    fn default() -> Self {
        Self {
            tracker_urls: Vec::new(),
            file_paths: Vec::new(),
            name: None,
            out_file: None,
            webseed_urls: Vec::new(),
            listen_port: 6881,
            upnp: false,
            ice_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            rtc_interval_ms: 5000,
            protocol: SeedProtocol::Both,
            version: TorrentVersion::V1,
            torrent_file: None,
            magnet: None,
            upload_limit: None,
            proxy: None,
            show_stats: true,
        }
    }
}