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
            version: TorrentVersion::V1,
            torrent_file: None,
            magnet: None,
        }
    }
}