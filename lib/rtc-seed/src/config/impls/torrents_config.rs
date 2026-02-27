use crate::config::structs::seeder_config::SeederConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

impl TorrentEntry {
    pub fn to_seeder_config(&self) -> Result<SeederConfig, String> {
        if self.file.is_empty() && self.torrent_file.is_none() {
            return Err("torrent entry needs at least one file or a torrent_file path".to_string());
        }
        let file_paths: Vec<PathBuf> = self.file.iter().map(PathBuf::from).collect();
        let out_file = self.out.as_ref().map(PathBuf::from);
        let webseed_urls = self.webseed.clone().unwrap_or_default();
        let ice_servers = self.ice.clone().unwrap_or_else(|| {
            vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ]
        });
        let rtc_interval_ms = self.rtc_interval.unwrap_or(5000);
        let version = match self.version.as_deref() {
            Some("v2") => TorrentVersion::V2,
            Some("hybrid") => TorrentVersion::Hybrid,
            _ => TorrentVersion::V1,
        };
        Ok(SeederConfig {
            tracker_urls: self.trackers.clone(),
            file_paths,
            name: self.name.clone(),
            out_file,
            webseed_urls,
            ice_servers,
            rtc_interval_ms,
            version,
            torrent_file: self.torrent_file.as_ref().map(PathBuf::from),
            magnet: self.magnet.clone(),
        })
    }
}
