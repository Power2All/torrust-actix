use crate::config::enums::seed_protocol::SeedProtocol;
use crate::config::structs::proxy_config::ProxyConfig;
use crate::config::structs::seeder_config::SeederConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

impl TorrentEntry {
    pub fn to_seeder_config(
        &self,
        proxy: Option<&ProxyConfig>,
        listen_port: u16,
        global_protocol: SeedProtocol,
        global_ice: &[String],
        global_rtc_interval_ms: u64,
    ) -> Result<SeederConfig, String> {
        if self.file.is_empty() && self.torrent_file.is_none() {
            return Err("torrent entry needs at least one file or a torrent_file path".to_string());
        }
        let file_paths: Vec<PathBuf> = self.file.iter().map(PathBuf::from).collect();
        let out_file = self.out.as_ref().map(PathBuf::from);
        let webseed_urls = self.webseed.clone().unwrap_or_default();
        let protocol = self.protocol.clone().unwrap_or(global_protocol);
        let ice_servers = self.ice.clone().unwrap_or_else(|| {
            if global_ice.is_empty() {
                vec![
                    "stun:stun.l.google.com:19302".to_string(),
                    "stun:stun1.l.google.com:19302".to_string(),
                ]
            } else {
                global_ice.to_vec()
            }
        });
        let rtc_interval_ms = self.rtc_interval
            .map(|s| s * 1000)
            .unwrap_or(global_rtc_interval_ms);
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
            upnp: false,
            ice_servers,
            rtc_interval_ms,
            protocol,
            version,
            torrent_file: self.torrent_file.as_ref().map(PathBuf::from),
            magnet: self.magnet.clone(),
            upload_limit: self.upload_limit,
            proxy: proxy.cloned(),
            show_stats: true,
        })
    }
}