use crate::config::structs::seeder_config::SeederConfig;
use crate::torrent::enums::torrent_version::TorrentVersion;
use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::structs::torrent_builder::TorrentBuilder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::torrent::torrent::{
    build_hybrid,
    build_v1,
    build_v2,
    torrent_creation_date
};
use std::io;

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
            files.push(FileEntry { path: p.clone(), name, length, offset: total_size });
            total_size += length;
        }
        let piece_length: u64 = if total_size <= 8 * 1024 * 1024 { 16 * 1024 } else { 32 * 1024 };
        let piece_count = (total_size as f64 / piece_length as f64).ceil() as usize;
        let name = config.name.clone().unwrap_or_else(|| {
            if files.len() == 1 {
                files[0].path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "torrent".to_string())
            } else {
                files[0].path.file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "torrent".to_string())
            }
        });
        let creation_date = torrent_creation_date();
        match config.version {
            TorrentVersion::V1 => {
                build_v1(config, files, total_size, piece_length, piece_count, name, creation_date)
            }
            TorrentVersion::V2 => {
                build_v2(config, files, total_size, piece_length, piece_count, name, creation_date)
            }
            TorrentVersion::Hybrid => {
                build_hybrid(config, files, total_size, piece_length, piece_count, name, creation_date)
            }
        }
    }
}