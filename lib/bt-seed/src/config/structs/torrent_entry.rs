use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TorrentEntry {
    pub out: Option<String>,
    pub name: Option<String>,
    pub file: Vec<String>,
    pub trackers: Vec<String>,
    pub webseed: Option<Vec<String>>,
    pub port: Option<u16>,
    pub version: Option<String>,
}