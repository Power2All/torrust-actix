use crate::config::enums::seed_protocol::SeedProtocol;
use crate::config::structs::proxy_config::ProxyConfig;
use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SeederConfig {
    pub tracker_urls: Vec<String>,
    pub file_paths: Vec<PathBuf>,
    pub name: Option<String>,
    pub out_file: Option<PathBuf>,
    pub webseed_urls: Vec<String>,
    pub listen_port: u16,
    pub upnp: bool,
    pub ice_servers: Vec<String>,
    pub rtc_interval_ms: u64,
    pub protocol: SeedProtocol,
    pub version: TorrentVersion,
    pub torrent_file: Option<PathBuf>,
    pub magnet: Option<String>,
    pub upload_limit: Option<u64>,
    pub proxy: Option<ProxyConfig>,
    pub show_stats: bool,
}