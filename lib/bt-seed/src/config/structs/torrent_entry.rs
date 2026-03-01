use serde::{
    Deserialize,
    Serialize
};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TorrentEntry {
    pub out: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub file: Vec<String>,
    #[serde(default)]
    pub trackers: Vec<String>,
    pub webseed: Option<Vec<String>>,
    pub version: Option<String>,
    pub torrent_file: Option<String>,
    pub magnet: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub upload_limit: Option<u64>,
}