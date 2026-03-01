use crate::config::structs::proxy_config::ProxyConfig;
use crate::config::structs::seeder_config::SeederConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

impl TorrentEntry {
    pub fn to_seeder_config(&self, proxy: Option<&ProxyConfig>, listen_port: u16) -> Result<SeederConfig, String> {
        if self.file.is_empty() && self.torrent_file.is_none() {
            return Err("torrent entry needs at least one file or a torrent_file path".to_string());
        }
        let file_paths: Vec<PathBuf> = self.file.iter().map(PathBuf::from).collect();
        let out_file = self.out.as_ref().map(PathBuf::from);
        let webseed_urls = self.webseed.clone().unwrap_or_default();
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
            listen_port,
            version,
            torrent_file: self.torrent_file.as_ref().map(PathBuf::from),
            magnet: self.magnet.clone(),
            upload_limit: self.upload_limit,
            proxy: proxy.cloned(),
            upnp: false,
            show_stats: true,
        })
    }
}