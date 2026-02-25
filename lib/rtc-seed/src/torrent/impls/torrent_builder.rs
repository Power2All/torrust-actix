use crate::config::structs::seeder_config::SeederConfig;
use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::structs::torrent_builder::TorrentBuilder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::torrent::torrent::{
    build_info_bencode,
    build_magnet_uri,
    build_torrent_bencode,
    hash_pieces,
};
use sha1::{
    Digest,
    Sha1
};
use std::io;
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

impl TorrentBuilder {
    pub fn build(config: &SeederConfig) -> io::Result<TorrentInfo> {
        assert!(!config.file_paths.is_empty(), "no files provided");
        let mut files: Vec<FileEntry> = Vec::new();
        let mut total_size: u64 = 0;
        for p in &config.file_paths {
            let meta = std::fs::metadata(p)?;
            let length = meta.len();
            let name: Vec<String> = p
                .file_name()
                .map(|n| vec![n.to_string_lossy().into_owned()])
                .unwrap_or_else(|| vec!["file".to_string()]);
            files.push(FileEntry {
                path: p.clone(),
                name,
                length,
                offset: total_size,
            });
            total_size += length;
        }
        let piece_length: u64 = if total_size <= 8 * 1024 * 1024 {
            16 * 1024
        } else {
            32 * 1024
        };
        let piece_count = (total_size as f64 / piece_length as f64).ceil() as usize;
        let pieces = hash_pieces(&files, piece_length, total_size, piece_count)?;
        let name = config.name.clone().unwrap_or_else(|| {
            if files.len() == 1 {
                files[0]
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "torrent".to_string())
            } else {
                files[0]
                    .path
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "torrent".to_string())
            }
        });
        let info_bytes = build_info_bencode(&name, piece_length, &pieces, &files, total_size);
        let info_hash: [u8; 20] = {
            let mut h = Sha1::new();
            h.update(&info_bytes);
            h.finalize().into()
        };
        let creation_date = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let torrent_bytes = build_torrent_bencode(
            &info_bytes,
            &config.tracker_url,
            creation_date,
            &config.webseed_urls,
        );
        let info_hash_hex = hex::encode(info_hash);
        let magnet_uri = build_magnet_uri(&info_hash_hex, &name, &config.tracker_url);
        Ok(TorrentInfo {
            name,
            piece_length,
            pieces,
            files,
            piece_count,
            total_size,
            info_hash,
            torrent_bytes,
            magnet_uri,
        })
    }
}