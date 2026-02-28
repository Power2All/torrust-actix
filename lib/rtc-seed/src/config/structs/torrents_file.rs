use crate::config::structs::global_config::GlobalConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TorrentsFile {
    pub torrents: Vec<TorrentEntry>,
    #[serde(default)]
    pub config: GlobalConfig,
}