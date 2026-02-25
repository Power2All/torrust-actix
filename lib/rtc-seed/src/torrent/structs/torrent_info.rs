use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::types::InfoHash;

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
}