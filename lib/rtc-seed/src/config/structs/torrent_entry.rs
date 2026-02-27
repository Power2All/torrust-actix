use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TorrentEntry {
    pub out: Option<String>,
    pub name: Option<String>,
    /// Files to seed. Required unless `torrent_file` is given.
    #[serde(default)]
    pub file: Vec<String>,
    /// Tracker URLs. Optional — may be read from `torrent_file` or `magnet`.
    #[serde(default)]
    pub trackers: Vec<String>,
    pub webseed: Option<Vec<String>>,
    pub ice: Option<Vec<String>>,
    pub rtc_interval: Option<u64>,
    pub version: Option<String>,
    /// Path to an existing .torrent file; trackers and info_hash are read from it.
    pub torrent_file: Option<String>,
    /// Magnet URI; tracker URLs are parsed from it.
    pub magnet: Option<String>,
}
