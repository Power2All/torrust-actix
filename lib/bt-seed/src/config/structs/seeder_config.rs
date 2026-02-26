use crate::torrent::enums::torrent_version::TorrentVersion;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SeederConfig {
    pub tracker_url: String,
    pub file_paths: Vec<PathBuf>,
    pub name: Option<String>,
    pub out_file: Option<PathBuf>,
    pub webseed_urls: Vec<String>,
    pub listen_port: u16,
    pub version: TorrentVersion,
}