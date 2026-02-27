use crate::torrent::enums::torrent_version::TorrentVersion;
use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::types::{
    InfoHash,
    V2InfoHash
};

#[derive(Debug, Clone)]
pub struct TorrentInfo {
    pub name: String,
    pub piece_length: u64,
    #[allow(dead_code)]
    pub pieces: Vec<u8>,
    pub files: Vec<FileEntry>,
    pub piece_count: usize,
    pub total_size: u64,
    pub info_hash: InfoHash,
    pub torrent_bytes: Vec<u8>,
    pub magnet_uri: String,
    #[allow(dead_code)]
    pub version: TorrentVersion,
    pub v2_info_hash: Option<V2InfoHash>,
    pub tracker_urls: Vec<String>,
}
