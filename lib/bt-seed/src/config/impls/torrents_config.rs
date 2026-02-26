use crate::config::structs::seeder_config::SeederConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

impl TorrentEntry {
    pub fn to_seeder_config(&self) -> Result<SeederConfig, String> {
        if self.file.is_empty() {
            return Err("torrent entry has no files".to_string());
        }
        if self.trackers.is_empty() {
            return Err("torrent entry has no trackers".to_string());
        }
        let tracker_url = self.trackers[0].clone();
        let file_paths: Vec<PathBuf> = self.file.iter().map(PathBuf::from).collect();
        let out_file = self.out.as_ref().map(PathBuf::from);
        let webseed_urls = self.webseed.clone().unwrap_or_default();
        let listen_port = self.port.unwrap_or(6881);
        let version = match self.version.as_deref() {
            Some("v2") => TorrentVersion::V2,
            Some("hybrid") => TorrentVersion::Hybrid,
            _ => TorrentVersion::V1,
        };
        Ok(SeederConfig {
            tracker_url,
            file_paths,
            name: self.name.clone(),
            out_file,
            webseed_urls,
            listen_port,
            version,
        })
    }
}