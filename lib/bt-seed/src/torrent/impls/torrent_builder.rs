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
        let mut tracker_urls: Vec<String> = config.tracker_urls.clone();
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
        if let Some(magnet_uri) = &config.magnet {
            let (mag_trackers, _mag_hash, _mag_name) = parse_magnet(magnet_uri);
            for url in mag_trackers {
                if !tracker_urls.contains(&url) {
                    tracker_urls.push(url);
                }
            }
        }
        assert!(!config.file_paths.is_empty(), "no files provided");
        // Compute name from the original paths before directory expansion
        let name = config.name.clone().unwrap_or_else(|| {
            config.file_paths[0]
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "torrent".to_string())
        });
        let mut files: Vec<FileEntry> = Vec::new();
        let mut total_size: u64 = 0;
        for p in &config.file_paths {
            let meta = std::fs::metadata(p)?;
            if meta.is_dir() {
                let mut dir_files: Vec<(std::path::PathBuf, Vec<String>)> = Vec::new();
                collect_dir_files(p, p, &mut dir_files)?;
                for (file_path, name_parts) in dir_files {
                    let length = std::fs::metadata(&file_path)?.len();
                    files.push(FileEntry { path: file_path, name: name_parts, length, offset: total_size });
                    total_size += length;
                }
            } else {
                let length = meta.len();
                let name_parts = p
                    .file_name()
                    .map(|n| vec![n.to_string_lossy().into_owned()])
                    .unwrap_or_else(|| vec!["file".to_string()]);
                files.push(FileEntry { path: p.clone(), name: name_parts, length, offset: total_size });
                total_size += length;
            }
        }
        let piece_length: u64 = if total_size <= 8 * 1024 * 1024 { 16 * 1024 } else { 32 * 1024 };
        let piece_count = (total_size as f64 / piece_length as f64).ceil() as usize;
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

fn collect_dir_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<(std::path::PathBuf, Vec<String>)>,
) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_dir_files(root, &path, out)?;
        } else {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let name_parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push((path, name_parts));
        }
    }
    Ok(())
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