use crate::config::structs::seeder_config::SeederConfig;
use crate::torrent::enums::torrent_version::TorrentVersion;
use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::structs::torrent_builder::TorrentBuilder;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::torrent::torrent::{
    build_hybrid,
    build_v1,
    build_v2,
    parse_magnet,
    parse_torrent_meta,
    torrent_creation_date
};
use std::io;

impl TorrentBuilder {
    pub fn build(config: &SeederConfig) -> io::Result<TorrentInfo> {
        // ── resolve tracker URLs ──────────────────────────────────────────
        let mut tracker_urls: Vec<String> = config.tracker_urls.clone();

        // If a torrent file is provided, parse it to get tracker URLs
        // and build TorrentInfo directly from it (no re-hashing).
        if let Some(torrent_path) = &config.torrent_file {
            let data = std::fs::read(torrent_path)
                .map_err(|e| io::Error::other(format!("reading {}: {}", torrent_path.display(), e)))?;
            let meta = parse_torrent_meta(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            for url in meta.tracker_urls {
                if !tracker_urls.contains(&url) {
                    tracker_urls.push(url);
                }
            }

            let files: Vec<FileEntry> = if !config.file_paths.is_empty() {
                config
                    .file_paths
                    .iter()
                    .zip(meta.files.iter())
                    .map(|(disk_path, parsed)| FileEntry {
                        path: disk_path.clone(),
                        name: parsed.name.clone(),
                        length: parsed.length,
                        offset: parsed.offset,
                    })
                    .collect()
            } else {
                meta.files
                    .into_iter()
                    .map(|mut f| {
                        if f.path.is_relative() {
                            f.path = std::env::current_dir()
                                .unwrap_or_default()
                                .join(&f.path);
                        }
                        f
                    })
                    .collect()
            };

            let name = config.name.clone().unwrap_or(meta.name);
            let magnet_uri = build_magnet_uri_simple(&hex::encode(meta.info_hash), &name, &tracker_urls);
            return Ok(TorrentInfo {
                name,
                piece_length: meta.piece_length,
                pieces: meta.pieces,
                files,
                piece_count: if meta.piece_length > 0 {
                    (meta.total_size as f64 / meta.piece_length as f64).ceil() as usize
                } else {
                    0
                },
                total_size: meta.total_size,
                info_hash: meta.info_hash,
                torrent_bytes: data,
                magnet_uri,
                version: TorrentVersion::V1,
                v2_info_hash: None,
                tracker_urls,
            });
        }

        // If a magnet URI is provided, parse its tracker URLs.
        if let Some(magnet_uri) = &config.magnet {
            let (mag_trackers, _mag_hash, _mag_name) = parse_magnet(magnet_uri);
            for url in mag_trackers {
                if !tracker_urls.contains(&url) {
                    tracker_urls.push(url);
                }
            }
        }

        // ── build from files ──────────────────────────────────────────────
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
                build_v1(&tracker_urls, &config.webseed_urls, files, total_size, piece_length, piece_count, name, creation_date)
            }
            TorrentVersion::V2 => {
                build_v2(&tracker_urls, &config.webseed_urls, files, total_size, piece_length, piece_count, name, creation_date)
            }
            TorrentVersion::Hybrid => {
                build_hybrid(&tracker_urls, &config.webseed_urls, files, total_size, piece_length, piece_count, name, creation_date)
            }
        }
    }
}

fn build_magnet_uri_simple(hash_hex: &str, name: &str, tracker_urls: &[String]) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    let encoded_name = utf8_percent_encode(name, NON_ALPHANUMERIC).to_string();
    let mut uri = format!("magnet:?xt=urn:btih:{}&dn={}", hash_hex, encoded_name);
    for url in tracker_urls {
        let encoded_tracker = utf8_percent_encode(url, NON_ALPHANUMERIC).to_string();
        uri.push_str("&tr=");
        uri.push_str(&encoded_tracker);
    }
    uri
}
