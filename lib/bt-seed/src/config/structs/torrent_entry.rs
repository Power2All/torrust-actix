use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TorrentEntry {
    pub out: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub file: Vec<String>,
    #[serde(default)]
    pub trackers: Vec<String>,
    pub webseed: Option<Vec<String>>,
    pub port: Option<u16>,
    pub version: Option<String>,
    pub torrent_file: Option<String>,
    pub magnet: Option<String>,
}