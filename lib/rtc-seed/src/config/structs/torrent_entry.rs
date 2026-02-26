use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TorrentEntry {
    pub out: Option<String>,
    pub name: Option<String>,
    pub file: Vec<String>,
    pub trackers: Vec<String>,
    pub webseed: Option<Vec<String>>,
    pub ice: Option<Vec<String>>,
    pub rtc_interval: Option<u64>,
    pub version: Option<String>,
}