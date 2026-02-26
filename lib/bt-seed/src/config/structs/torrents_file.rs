use crate::config::structs::torrent_entry::TorrentEntry;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TorrentsFile {
    pub torrents: Vec<TorrentEntry>,
}