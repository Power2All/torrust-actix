use crate::config::structs::seeder_config::SeederConfig;
use crate::torrent::enums::torrent_version::TorrentVersion;

impl Default for SeederConfig {
    fn default() -> Self {
        Self {
            tracker_url: "http://127.0.0.1:6969/announce".to_string(),
            file_paths: Vec::new(),
            name: None,
            out_file: None,
            webseed_urls: Vec::new(),
            listen_port: 6881,
            version: TorrentVersion::V1,
        }
    }
}