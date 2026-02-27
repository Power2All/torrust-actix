use crate::config::structs::seeder_config::SeederConfig;
use crate::torrent::enums::torrent_version::TorrentVersion;
use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::torrent::types::QUERY_ENCODE;
use percent_encoding::utf8_percent_encode;
use sha1::{
    Digest,
    Sha1
};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{
    self,
    Read
};
use std::path::Path;
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

pub fn hash_pieces(
    files: &[FileEntry],
    piece_length: u64,
    total_size: u64,
    piece_count: usize,
) -> io::Result<Vec<u8>> {
    let mut all_hashes: Vec<u8> = Vec::with_capacity(piece_count * 20);
    let mut file_idx = 0usize;
    let mut file_offset: u64 = 0;
    let mut current_file: Option<File> = if !files.is_empty() {
        Some(File::open(&files[0].path)?)
    } else {
        None
    };
    let mut piece_buf: Vec<u8> = vec![0u8; piece_length as usize];
    for piece_i in 0..piece_count {
        let piece_start = piece_i as u64 * piece_length;
        let piece_end = (piece_start + piece_length).min(total_size);
        let this_piece_len = (piece_end - piece_start) as usize;
        let buf = &mut piece_buf[..this_piece_len];
        let mut filled = 0usize;
        while filled < this_piece_len {
            let f = match current_file.as_mut() {
                Some(f) => f,
                None => break,
            };
            let remaining_in_file = files[file_idx].length - file_offset;
            let need = (this_piece_len - filled) as u64;
            let to_read = need.min(remaining_in_file) as usize;
            let n = f.read(&mut buf[filled..filled + to_read])?;
            if n == 0 {
                break;
            }
            filled += n;
            file_offset += n as u64;
            if file_offset >= files[file_idx].length {
                file_idx += 1;
                file_offset = 0;
                current_file = if file_idx < files.len() {
                    Some(File::open(&files[file_idx].path)?)
                } else {
                    None
                };
            }
        }
        let mut h = Sha1::new();
        h.update(&buf[..filled]);
        let hash: [u8; 20] = h.finalize().into();
        all_hashes.extend_from_slice(&hash);
    }
    Ok(all_hashes)
}

pub fn build_info_bencode(
    name: &str,
    piece_length: u64,
    pieces: &[u8],
    files: &[FileEntry],
    total_size: u64,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"d");
    if files.len() > 1 {
        out.extend_from_slice(b"5:filesl");
        for f in files {
            out.extend_from_slice(b"d");
            out.extend_from_slice(b"6:lengthi");
            out.extend_from_slice(f.length.to_string().as_bytes());
            out.extend_from_slice(b"e");
            out.extend_from_slice(b"4:pathl");
            for component in &f.name {
                write_bencode_string(&mut out, component.as_bytes());
            }
            out.extend_from_slice(b"e");
            out.extend_from_slice(b"e");
        }
        out.extend_from_slice(b"e");
    } else {
        out.extend_from_slice(b"6:lengthi");
        out.extend_from_slice(total_size.to_string().as_bytes());
        out.extend_from_slice(b"e");
    }
    out.extend_from_slice(b"4:name");
    write_bencode_string(&mut out, name.as_bytes());
    out.extend_from_slice(b"12:piece lengthi");
    out.extend_from_slice(piece_length.to_string().as_bytes());
    out.extend_from_slice(b"e");
    out.extend_from_slice(b"6:pieces");
    out.extend_from_slice(pieces.len().to_string().as_bytes());
    out.extend_from_slice(b":");
    out.extend_from_slice(pieces);
    out.extend_from_slice(b"e");
    out
}

pub fn build_torrent_bencode(
    info_bytes: &[u8],
    tracker_url: &str,
    creation_date: u64,
    webseed_urls: &[String],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"d");
    out.extend_from_slice(b"8:announce");
    write_bencode_string(&mut out, tracker_url.as_bytes());
    let created_by = "Torrust-Actix bt-seed v0.1";
    out.extend_from_slice(b"10:created by");
    write_bencode_string(&mut out, created_by.as_bytes());
    out.extend_from_slice(b"13:creation datei");
    out.extend_from_slice(creation_date.to_string().as_bytes());
    out.extend_from_slice(b"e");
    out.extend_from_slice(b"4:info");
    out.extend_from_slice(info_bytes);
    if !webseed_urls.is_empty() {
        out.extend_from_slice(b"8:url-list");
        if webseed_urls.len() == 1 {
            write_bencode_string(&mut out, webseed_urls[0].as_bytes());
        } else {
            out.extend_from_slice(b"l");
            for url in webseed_urls {
                write_bencode_string(&mut out, url.as_bytes());
            }
            out.extend_from_slice(b"e");
        }
    }
    out.extend_from_slice(b"e");
    out
}

pub fn write_bencode_string(out: &mut Vec<u8>, s: &[u8]) {
    out.extend_from_slice(s.len().to_string().as_bytes());
    out.extend_from_slice(b":");
    out.extend_from_slice(s);
}

pub fn build_magnet_uri(info_hash_hex: &str, name: &str, tracker_url: &str) -> String {
    let encoded_name = utf8_percent_encode(name, QUERY_ENCODE).to_string();
    let encoded_tracker = utf8_percent_encode(tracker_url, QUERY_ENCODE).to_string();
    format!(
        "magnet:?xt=urn:btih:{}&dn={}&tr={}",
        info_hash_hex, encoded_name, encoded_tracker
    )
}

pub fn sha256_block(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

fn next_power_of_2(n: usize) -> usize {
    if n == 0 { return 1; }
    let mut p = 1usize;
    while p < n { p <<= 1; }
    p
}

fn merkle_layer_up(nodes: &[[u8; 32]]) -> Vec<[u8; 32]> {
    debug_assert_eq!(nodes.len() % 2, 0);
    let mut out = Vec::with_capacity(nodes.len() / 2);
    for i in (0..nodes.len()).step_by(2) {
        let mut pair = [0u8; 64];
        pair[..32].copy_from_slice(&nodes[i]);
        pair[32..].copy_from_slice(&nodes[i + 1]);
        out.push(sha256_block(&pair));
    }
    out
}

pub fn build_merkle_tree_and_layers(
    file_path: &Path,
    file_size: u64,
    piece_length: u64,
) -> io::Result<([u8; 32], Vec<[u8; 32]>)> {
    const BLOCK_SIZE: u64 = 16 * 1024;
    if file_size == 0 {
        return Ok(([0u8; 32], Vec::new()));
    }
    let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
    let mut file = File::open(file_path)?;
    let mut buf = vec![0u8; BLOCK_SIZE as usize];
    let mut remaining = file_size;
    while remaining > 0 {
        let to_read = remaining.min(BLOCK_SIZE) as usize;
        file.read_exact(&mut buf[..to_read])?;
        if to_read < BLOCK_SIZE as usize {
            buf[to_read..].fill(0);
        }
        leaf_hashes.push(sha256_block(&buf));
        remaining -= to_read as u64;
    }
    let block_count = leaf_hashes.len();
    let padded_count = next_power_of_2(block_count);
    let mut current_layer: Vec<[u8; 32]> = leaf_hashes.clone();
    while current_layer.len() < padded_count {
        current_layer.push([0u8; 32]);
    }
    let blocks_per_piece = (piece_length / BLOCK_SIZE).max(1) as usize;
    let level_offset = blocks_per_piece.trailing_zeros() as usize;
    let mut piece_layer: Option<Vec<[u8; 32]>> = None;
    let mut level = 0usize;
    loop {
        if level == level_offset {
            let real_piece_count = block_count.div_ceil(blocks_per_piece).max(1);
            piece_layer = Some(current_layer[..real_piece_count].to_vec());
        }
        if current_layer.len() == 1 {
            break;
        }
        current_layer = merkle_layer_up(&current_layer);
        level += 1;
    }
    if piece_layer.is_none() {
        piece_layer = Some(vec![current_layer[0]]);
    }
    let root = current_layer[0];
    let piece_layer = piece_layer.unwrap();
    let layer_hashes = if file_size > piece_length { piece_layer } else { Vec::new() };
    Ok((root, layer_hashes))
}

enum FtNode {
    Leaf { length: u64, root: Option<[u8; 32]> },
    Dir(BTreeMap<String, FtNode>),
}

fn ft_insert(tree: &mut BTreeMap<String, FtNode>, components: &[String], length: u64, root: Option<[u8; 32]>) {
    if components.is_empty() { return; }
    let key = components[0].clone();
    if components.len() == 1 {
        tree.insert(key, FtNode::Leaf { length, root });
    } else {
        let child = tree.entry(key).or_insert_with(|| FtNode::Dir(BTreeMap::new()));
        if let FtNode::Dir(sub) = child {
            ft_insert(sub, &components[1..], length, root);
        }
    }
}

fn ft_encode(out: &mut Vec<u8>, node: &FtNode) {
    match node {
        FtNode::Leaf { length, root } => {
            out.push(b'd');
            out.extend_from_slice(b"0:");
            out.push(b'd');
            out.extend_from_slice(b"6:length");
            out.push(b'i');
            out.extend_from_slice(length.to_string().as_bytes());
            out.push(b'e');
            if let Some(r) = root
                && *length > 0 {
                    out.extend_from_slice(b"11:pieces root");
                    write_bencode_string(out, r);
                }
            out.push(b'e');
            out.push(b'e');
        }
        FtNode::Dir(children) => {
            out.push(b'd');
            for (name, child) in children {
                write_bencode_string(out, name.as_bytes());
                ft_encode(out, child);
            }
            out.push(b'e');
        }
    }
}

pub fn build_v2_file_tree_bencode(files: &[FileEntry], roots: &[[u8; 32]]) -> Vec<u8> {
    let mut tree: BTreeMap<String, FtNode> = BTreeMap::new();
    for (i, file) in files.iter().enumerate() {
        let root = if file.length > 0 { Some(roots[i]) } else { None };
        ft_insert(&mut tree, &file.name, file.length, root);
    }
    let mut out = Vec::new();
    out.push(b'd');
    for (name, node) in &tree {
        write_bencode_string(&mut out, name.as_bytes());
        ft_encode(&mut out, node);
    }
    out.push(b'e');
    out
}

pub fn build_piece_layers_bencode(layers: &[([u8; 32], Vec<[u8; 32]>)]) -> Vec<u8> {
    let mut sorted: Vec<&([u8; 32], Vec<[u8; 32]>)> = layers.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    out.push(b'd');
    for (root, hashes) in sorted {
        write_bencode_string(&mut out, root);
        let value: Vec<u8> = hashes.iter().flat_map(|h| h.iter().copied()).collect();
        write_bencode_string(&mut out, &value);
    }
    out.push(b'e');
    out
}

pub fn build_v2_info_bencode(
    name: &str,
    piece_length: u64,
    files: &[FileEntry],
    roots: &[[u8; 32]],
) -> Vec<u8> {
    let file_tree = build_v2_file_tree_bencode(files, roots);
    let mut out = Vec::new();
    out.push(b'd');
    out.extend_from_slice(b"9:file tree");
    out.extend_from_slice(&file_tree);
    out.extend_from_slice(b"12:meta versioni2e");
    out.extend_from_slice(b"4:name");
    write_bencode_string(&mut out, name.as_bytes());
    out.extend_from_slice(b"12:piece length");
    out.push(b'i');
    out.extend_from_slice(piece_length.to_string().as_bytes());
    out.push(b'e');
    out.push(b'e');
    out
}

pub fn build_hybrid_info_bencode(
    name: &str,
    piece_length: u64,
    pieces_v1: &[u8],
    files: &[FileEntry],
    roots: &[[u8; 32]],
    total_size: u64,
) -> Vec<u8> {
    let file_tree = build_v2_file_tree_bencode(files, roots);
    let mut out = Vec::new();
    out.push(b'd');
    out.extend_from_slice(b"9:file tree");
    out.extend_from_slice(&file_tree);
    if files.len() > 1 {
        out.extend_from_slice(b"5:files");
        out.push(b'l');
        for f in files {
            out.push(b'd');
            out.extend_from_slice(b"6:length");
            out.push(b'i');
            out.extend_from_slice(f.length.to_string().as_bytes());
            out.push(b'e');
            out.extend_from_slice(b"4:path");
            out.push(b'l');
            for component in &f.name {
                write_bencode_string(&mut out, component.as_bytes());
            }
            out.push(b'e');
            out.push(b'e');
        }
        out.push(b'e');
    } else {
        out.extend_from_slice(b"6:length");
        out.push(b'i');
        out.extend_from_slice(total_size.to_string().as_bytes());
        out.push(b'e');
    }
    out.extend_from_slice(b"12:meta versioni2e");
    out.extend_from_slice(b"4:name");
    write_bencode_string(&mut out, name.as_bytes());
    out.extend_from_slice(b"12:piece length");
    out.push(b'i');
    out.extend_from_slice(piece_length.to_string().as_bytes());
    out.push(b'e');
    out.extend_from_slice(b"6:pieces");
    out.extend_from_slice(pieces_v1.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(pieces_v1);
    out.push(b'e');
    out
}

pub fn build_v2_torrent_bencode(
    info_bytes: &[u8],
    tracker_url: &str,
    creation_date: u64,
    webseed_urls: &[String],
    piece_layers_bytes: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(b'd');
    out.extend_from_slice(b"8:announce");
    write_bencode_string(&mut out, tracker_url.as_bytes());
    let created_by = b"Torrust-Actix bt-seed v0.1";
    out.extend_from_slice(b"10:created by");
    write_bencode_string(&mut out, created_by);
    out.extend_from_slice(b"13:creation date");
    out.push(b'i');
    out.extend_from_slice(creation_date.to_string().as_bytes());
    out.push(b'e');
    out.extend_from_slice(b"4:info");
    out.extend_from_slice(info_bytes);
    out.extend_from_slice(b"12:piece layers");
    out.extend_from_slice(piece_layers_bytes);
    if !webseed_urls.is_empty() {
        out.extend_from_slice(b"8:url-list");
        if webseed_urls.len() == 1 {
            write_bencode_string(&mut out, webseed_urls[0].as_bytes());
        } else {
            out.push(b'l');
            for url in webseed_urls {
                write_bencode_string(&mut out, url.as_bytes());
            }
            out.push(b'e');
        }
    }
    out.push(b'e');
    out
}

pub fn build_v2_magnet_uri(v2_hash_hex: &str, name: &str, tracker_url: &str) -> String {
    let encoded_name = utf8_percent_encode(name, QUERY_ENCODE).to_string();
    let encoded_tracker = utf8_percent_encode(tracker_url, QUERY_ENCODE).to_string();
    format!(
        "magnet:?xt=urn:btmh:1220{}&dn={}&tr={}",
        v2_hash_hex, encoded_name, encoded_tracker
    )
}

pub fn build_hybrid_magnet_uri(v1_hex: &str, v2_hex: &str, name: &str, tracker_url: &str) -> String {
    let encoded_name = utf8_percent_encode(name, QUERY_ENCODE).to_string();
    let encoded_tracker = utf8_percent_encode(tracker_url, QUERY_ENCODE).to_string();
    format!(
        "magnet:?xt=urn:btih:{}&xt=urn:btmh:1220{}&dn={}&tr={}",
        v1_hex, v2_hex, encoded_name, encoded_tracker
    )
}

pub fn build_v1(
    config: &SeederConfig,
    files: Vec<FileEntry>,
    total_size: u64,
    piece_length: u64,
    piece_count: usize,
    name: String,
    creation_date: u64,
) -> io::Result<TorrentInfo> {
    let pieces = hash_pieces(&files, piece_length, total_size, piece_count)?;
    let info_bytes = build_info_bencode(&name, piece_length, &pieces, &files, total_size);
    let info_hash: [u8; 20] = {
        let mut h = Sha1::new();
        h.update(&info_bytes);
        h.finalize().into()
    };
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
        version: TorrentVersion::V1,
        v2_info_hash: None,
    })
}

pub fn build_v2(
    config: &SeederConfig,
    files: Vec<FileEntry>,
    total_size: u64,
    piece_length: u64,
    piece_count: usize,
    name: String,
    creation_date: u64,
) -> io::Result<TorrentInfo> {
    let mut roots: Vec<[u8; 32]> = Vec::new();
    let mut piece_layers: Vec<([u8; 32], Vec<[u8; 32]>)> = Vec::new();
    for file in &files {
        let (root, layer_hashes) =
            build_merkle_tree_and_layers(&file.path, file.length, piece_length)?;
        if !layer_hashes.is_empty() {
            piece_layers.push((root, layer_hashes));
        }
        roots.push(root);
    }
    let info_bytes = build_v2_info_bencode(&name, piece_length, &files, &roots);
    let piece_layers_bytes = build_piece_layers_bencode(&piece_layers);
    let v2_hash: [u8; 32] = {
        let mut h = Sha256::new();
        h.update(&info_bytes);
        h.finalize().into()
    };
    let info_hash: [u8; 20] = v2_hash[..20].try_into().unwrap();
    let torrent_bytes = build_v2_torrent_bencode(
        &info_bytes,
        &config.tracker_url,
        creation_date,
        &config.webseed_urls,
        &piece_layers_bytes,
    );
    let v2_hash_hex = hex::encode(v2_hash);
    let magnet_uri = build_v2_magnet_uri(&v2_hash_hex, &name, &config.tracker_url);
    Ok(TorrentInfo {
        name,
        piece_length,
        pieces: Vec::new(),
        files,
        piece_count,
        total_size,
        info_hash,
        torrent_bytes,
        magnet_uri,
        version: TorrentVersion::V2,
        v2_info_hash: Some(v2_hash),
    })
}

pub fn build_hybrid(
    config: &SeederConfig,
    files: Vec<FileEntry>,
    total_size: u64,
    piece_length: u64,
    piece_count: usize,
    name: String,
    creation_date: u64,
) -> io::Result<TorrentInfo> {
    let pieces = hash_pieces(&files, piece_length, total_size, piece_count)?;
    let mut roots: Vec<[u8; 32]> = Vec::new();
    let mut piece_layers: Vec<([u8; 32], Vec<[u8; 32]>)> = Vec::new();
    for file in &files {
        let (root, layer_hashes) =
            build_merkle_tree_and_layers(&file.path, file.length, piece_length)?;
        if !layer_hashes.is_empty() {
            piece_layers.push((root, layer_hashes));
        }
        roots.push(root);
    }
    let info_bytes =
        build_hybrid_info_bencode(&name, piece_length, &pieces, &files, &roots, total_size);
    let piece_layers_bytes = build_piece_layers_bencode(&piece_layers);
    let info_hash: [u8; 20] = {
        let mut h = Sha1::new();
        h.update(&info_bytes);
        h.finalize().into()
    };
    let v2_hash: [u8; 32] = {
        let mut h = Sha256::new();
        h.update(&info_bytes);
        h.finalize().into()
    };
    let torrent_bytes = build_v2_torrent_bencode(
        &info_bytes,
        &config.tracker_url,
        creation_date,
        &config.webseed_urls,
        &piece_layers_bytes,
    );
    let v1_hex = hex::encode(info_hash);
    let v2_hex = hex::encode(v2_hash);
    let magnet_uri = build_hybrid_magnet_uri(&v1_hex, &v2_hex, &name, &config.tracker_url);
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
        version: TorrentVersion::Hybrid,
        v2_info_hash: Some(v2_hash),
    })
}

pub fn torrent_creation_date() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}