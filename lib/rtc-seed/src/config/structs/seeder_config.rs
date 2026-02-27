use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SeederConfig {
    /// Explicit tracker URLs provided via CLI/YAML. Empty = none given.
    pub tracker_urls: Vec<String>,
    pub file_paths: Vec<PathBuf>,
    pub name: Option<String>,
    pub out_file: Option<PathBuf>,
    pub webseed_urls: Vec<String>,
    pub ice_servers: Vec<String>,
    pub rtc_interval_ms: u64,
    pub version: TorrentVersion,
    /// Path to an existing .torrent file — trackers (and info_hash) are read from it.
    pub torrent_file: Option<PathBuf>,
    /// Magnet URI — tracker URLs (and optionally info_hash) are parsed from it.
    pub magnet: Option<String>,
}
