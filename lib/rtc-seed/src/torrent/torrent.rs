use crate::torrent::enums::torrent_version::TorrentVersion;
use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::structs::torrent_info::TorrentInfo;
use crate::torrent::types::QUERY_ENCODE;
use percent_encoding::{
    percent_decode_str,
    utf8_percent_encode
};
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
use std::path::{
    Path,
    PathBuf
};
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

const CREATED_BY: &str = concat!("Torrust-Actix rtc-seed v", env!("CARGO_PKG_VERSION"));

fn bencode_end(data: &[u8], pos: usize) -> Result<usize, String> {
    if pos >= data.len() {
        return Err(format!("bencode_end: pos {} out of range (len {})", pos, data.len()));
    }
    match data[pos] {
        b'i' => {
            let rel = data[pos..].iter().position(|&b| b == b'e').ok_or("unterminated integer")?;
            Ok(pos + rel + 1)
        }
        b'l' => {
            let mut p = pos + 1;
            while p < data.len() && data[p] != b'e' {
                p = bencode_end(data, p)?;
            }
            if p >= data.len() { return Err("unterminated list".to_string()); }
            Ok(p + 1)
        }
        b'd' => {
            let mut p = pos + 1;
            while p < data.len() && data[p] != b'e' {
                p = bencode_end(data, p)?;
                if p >= data.len() || data[p] == b'e' { break; }
                p = bencode_end(data, p)?;
            }
            if p >= data.len() { return Err("unterminated dict".to_string()); }
            Ok(p + 1)
        }
        b'0'..=b'9' => {
            let colon = data[pos..].iter().position(|&b| b == b':').ok_or("no colon in bstring")?;
            let len: usize = std::str::from_utf8(&data[pos..pos + colon])
                .map_err(|e| e.to_string())?
                .parse()
                .map_err(|e: std::num::ParseIntError| e.to_string())?;
            let end = pos + colon + 1 + len;
            if end > data.len() { return Err("bstring out of bounds".to_string()); }
            Ok(end)
        }
        b => Err(format!("unexpected bencode byte 0x{:02x} at pos {}", b, pos)),
    }
}

fn read_bstring(data: &[u8], pos: usize) -> Result<(&[u8], usize), String> {
    let colon = data[pos..].iter().position(|&b| b == b':').ok_or("no colon")?;
    let len: usize = std::str::from_utf8(&data[pos..pos + colon])
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let start = pos + colon + 1;
    let end = start + len;
    if end > data.len() { return Err("bstring data out of bounds".to_string()); }
    Ok((&data[start..end], end))
}

fn read_bint(data: &[u8], pos: usize) -> Result<(i64, usize), String> {
    if pos >= data.len() || data[pos] != b'i' {
        return Err(format!("expected 'i' at pos {}", pos));
    }
    let rel = data[pos..].iter().position(|&b| b == b'e').ok_or("unterminated integer")?;
    let s = std::str::from_utf8(&data[pos + 1..pos + rel]).map_err(|e| e.to_string())?;
    let n: i64 = s.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    Ok((n, pos + rel + 1))
}

pub struct ParsedTorrentMeta {
    pub info_hash: [u8; 20],
    pub tracker_urls: Vec<String>,
    pub name: String,
    pub piece_length: u64,
    pub pieces: Vec<u8>,
    pub files: Vec<FileEntry>,
    pub total_size: u64,
}

pub fn parse_torrent_meta(data: &[u8]) -> Result<ParsedTorrentMeta, String> {
    if data.is_empty() || data[0] != b'd' {
        return Err("torrent file must start with a bencode dict".to_string());
    }
    let mut tracker_urls: Vec<String> = Vec::new();
    let mut info_raw: Option<&[u8]> = None;
    let mut pos = 1usize;
    while pos < data.len() && data[pos] != b'e' {
        let (key, after_key) = read_bstring(data, pos)?;
        pos = after_key;
        match key {
            b"announce" => {
                let (val, after) = read_bstring(data, pos)?;
                pos = after;
                let url = String::from_utf8_lossy(val).into_owned();
                if !url.is_empty() && !tracker_urls.contains(&url) {
                    tracker_urls.insert(0, url);
                }
            }
            b"announce-list" => {
                if pos >= data.len() || data[pos] != b'l' {
                    pos = bencode_end(data, pos)?;
                    continue;
                }
                pos += 1;
                while pos < data.len() && data[pos] != b'e' {
                    if data[pos] != b'l' {
                        pos = bencode_end(data, pos)?;
                        continue;
                    }
                    pos += 1;
                    while pos < data.len() && data[pos] != b'e' {
                        if data[pos].is_ascii_digit() {
                            let (val, after) = read_bstring(data, pos)?;
                            pos = after;
                            let url = String::from_utf8_lossy(val).into_owned();
                            if !url.is_empty() && !tracker_urls.contains(&url) {
                                tracker_urls.push(url);
                            }
                        } else {
                            pos = bencode_end(data, pos)?;
                        }
                    }
                    if pos < data.len() && data[pos] == b'e' { pos += 1; }
                }
                if pos < data.len() && data[pos] == b'e' { pos += 1; }
            }
            b"info" => {
                let info_start = pos;
                let info_end = bencode_end(data, pos)?;
                info_raw = Some(&data[info_start..info_end]);
                pos = info_end;
            }
            _ => {
                pos = bencode_end(data, pos)?;
            }
        }
    }
    let info_bytes = info_raw.ok_or("no info dict in torrent")?;
    let info_hash: [u8; 20] = {
        let mut h = Sha1::new();
        h.update(info_bytes);
        h.finalize().into()
    };
    if info_bytes[0] != b'd' {
        return Err("info value is not a dict".to_string());
    }
    let mut name = String::new();
    let mut piece_length: u64 = 0;
    let mut pieces: Vec<u8> = Vec::new();
    let mut single_length: Option<u64> = None;
    let mut multi_files: Vec<(u64, Vec<String>)> = Vec::new();
    let mut ipos = 1usize;
    while ipos < info_bytes.len() && info_bytes[ipos] != b'e' {
        let (key, after_key) = read_bstring(info_bytes, ipos)?;
        ipos = after_key;
        match key {
            b"name" => {
                let (val, after) = read_bstring(info_bytes, ipos)?;
                ipos = after;
                name = String::from_utf8_lossy(val).into_owned();
            }
            b"piece length" => {
                let (val, after) = read_bint(info_bytes, ipos)?;
                ipos = after;
                piece_length = val as u64;
            }
            b"pieces" => {
                let (val, after) = read_bstring(info_bytes, ipos)?;
                ipos = after;
                pieces = val.to_vec();
            }
            b"length" => {
                let (val, after) = read_bint(info_bytes, ipos)?;
                ipos = after;
                single_length = Some(val as u64);
            }
            b"files" => {
                if ipos >= info_bytes.len() || info_bytes[ipos] != b'l' {
                    ipos = bencode_end(info_bytes, ipos)?;
                    continue;
                }
                ipos += 1;
                while ipos < info_bytes.len() && info_bytes[ipos] != b'e' {
                    if info_bytes[ipos] != b'd' {
                        ipos = bencode_end(info_bytes, ipos)?;
                        continue;
                    }
                    ipos += 1;
                    let mut flen: u64 = 0;
                    let mut path_comps: Vec<String> = Vec::new();
                    while ipos < info_bytes.len() && info_bytes[ipos] != b'e' {
                        let (fkey, after_fkey) = read_bstring(info_bytes, ipos)?;
                        ipos = after_fkey;
                        match fkey {
                            b"length" => {
                                let (v, after) = read_bint(info_bytes, ipos)?;
                                ipos = after;
                                flen = v as u64;
                            }
                            b"path" => {
                                if ipos >= info_bytes.len() || info_bytes[ipos] != b'l' {
                                    ipos = bencode_end(info_bytes, ipos)?;
                                    continue;
                                }
                                ipos += 1;
                                while ipos < info_bytes.len() && info_bytes[ipos] != b'e' {
                                    let (comp, after) = read_bstring(info_bytes, ipos)?;
                                    ipos = after;
                                    path_comps.push(String::from_utf8_lossy(comp).into_owned());
                                }
                                if ipos < info_bytes.len() && info_bytes[ipos] == b'e' { ipos += 1; }
                            }
                            _ => { ipos = bencode_end(info_bytes, ipos)?; }
                        }
                    }
                    if ipos < info_bytes.len() && info_bytes[ipos] == b'e' { ipos += 1; }
                    multi_files.push((flen, path_comps));
                }
                if ipos < info_bytes.len() && info_bytes[ipos] == b'e' { ipos += 1; }
            }
            _ => { ipos = bencode_end(info_bytes, ipos)?; }
        }
    }
    let (files, total_size) = if !multi_files.is_empty() {
        let mut offset = 0u64;
        let mut total = 0u64;
        let entries: Vec<FileEntry> = multi_files
            .into_iter()
            .map(|(flen, comps)| {
                let mut p = PathBuf::from(&name);
                for c in &comps { p.push(c); }
                let entry = FileEntry { path: p, name: comps, length: flen, offset };
                offset += flen;
                total += flen;
                entry
            })
            .collect();
        (entries, total)
    } else {
        let len = single_length.unwrap_or(0);
        (
            vec![FileEntry {
                path: PathBuf::from(&name),
                name: vec![name.clone()],
                length: len,
                offset: 0,
            }],
            len,
        )
    };
    Ok(ParsedTorrentMeta { info_hash, tracker_urls, name, piece_length, pieces, files, total_size })
}

pub fn parse_magnet(uri: &str) -> (Vec<String>, Option<[u8; 20]>, Option<String>) {
    let mut trackers: Vec<String> = Vec::new();
    let mut info_hash: Option<[u8; 20]> = None;
    let mut name: Option<String> = None;
    let query = match uri.strip_prefix("magnet:?") {
        Some(q) => q,
        None => return (trackers, info_hash, name),
    };
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("tr=") {
            let decoded = percent_decode_str(value).decode_utf8_lossy().into_owned();
            if !decoded.is_empty() && !trackers.contains(&decoded) {
                trackers.push(decoded);
            }
        } else if let Some(value) = param.strip_prefix("xt=urn:btih:") {
            if value.len() == 40
                && let Ok(bytes) = hex::decode(value)
                && let Ok(arr) = bytes.try_into()
            {
                info_hash = Some(arr);
            }
        } else if let Some(value) = param.strip_prefix("dn=") {
            let decoded = percent_decode_str(value).decode_utf8_lossy().into_owned();
            if !decoded.is_empty() {
                name = Some(decoded);
            }
        }
    }
    (trackers, info_hash, name)
}

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
    tracker_urls: &[String],
    creation_date: u64,
    webseed_urls: &[String],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"d");
    let first = tracker_urls.first().map(|s| s.as_str()).unwrap_or("");
    out.extend_from_slice(b"8:announce");
    write_bencode_string(&mut out, first.as_bytes());
    if !tracker_urls.is_empty() {
        out.extend_from_slice(b"13:announce-listl");
        out.push(b'l');
        for url in tracker_urls {
            write_bencode_string(&mut out, url.as_bytes());
        }
        out.push(b'e');
        out.push(b'e');
    }
    out.extend_from_slice(b"10:created by");
    write_bencode_string(&mut out, CREATED_BY.as_bytes());
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

pub fn build_magnet_uri(info_hash_hex: &str, name: &str, tracker_urls: &[String]) -> String {
    let encoded_name = utf8_percent_encode(name, QUERY_ENCODE).to_string();
    let mut uri = format!("magnet:?xt=urn:btih:{}&dn={}", info_hash_hex, encoded_name);
    for url in tracker_urls {
        let encoded_tracker = utf8_percent_encode(url, QUERY_ENCODE).to_string();
        uri.push_str("&tr=");
        uri.push_str(&encoded_tracker);
    }
    uri
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
    tracker_urls: &[String],
    creation_date: u64,
    webseed_urls: &[String],
    piece_layers_bytes: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(b'd');
    let first = tracker_urls.first().map(|s| s.as_str()).unwrap_or("");
    out.extend_from_slice(b"8:announce");
    write_bencode_string(&mut out, first.as_bytes());
    if !tracker_urls.is_empty() {
        out.extend_from_slice(b"13:announce-listl");
        out.push(b'l');
        for url in tracker_urls {
            write_bencode_string(&mut out, url.as_bytes());
        }
        out.push(b'e');
        out.push(b'e');
    }
    out.extend_from_slice(b"10:created by");
    write_bencode_string(&mut out, CREATED_BY.as_bytes());
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

pub fn build_v2_magnet_uri(v2_hash_hex: &str, name: &str, tracker_urls: &[String]) -> String {
    let encoded_name = utf8_percent_encode(name, QUERY_ENCODE).to_string();
    let mut uri = format!("magnet:?xt=urn:btmh:1220{}&dn={}", v2_hash_hex, encoded_name);
    for url in tracker_urls {
        let encoded_tracker = utf8_percent_encode(url, QUERY_ENCODE).to_string();
        uri.push_str("&tr=");
        uri.push_str(&encoded_tracker);
    }
    uri
}

pub fn build_hybrid_magnet_uri(v1_hex: &str, v2_hex: &str, name: &str, tracker_urls: &[String]) -> String {
    let encoded_name = utf8_percent_encode(name, QUERY_ENCODE).to_string();
    let mut uri = format!("magnet:?xt=urn:btih:{}&xt=urn:btmh:1220{}&dn={}", v1_hex, v2_hex, encoded_name);
    for url in tracker_urls {
        let encoded_tracker = utf8_percent_encode(url, QUERY_ENCODE).to_string();
        uri.push_str("&tr=");
        uri.push_str(&encoded_tracker);
    }
    uri
}

#[allow(clippy::too_many_arguments)]
pub fn build_v1(
    tracker_urls: &[String],
    webseed_urls: &[String],
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
    let torrent_bytes = build_torrent_bencode(&info_bytes, tracker_urls, creation_date, webseed_urls);
    let info_hash_hex = hex::encode(info_hash);
    let magnet_uri = build_magnet_uri(&info_hash_hex, &name, tracker_urls);
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
        tracker_urls: tracker_urls.to_vec(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn build_v2(
    tracker_urls: &[String],
    webseed_urls: &[String],
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
    let torrent_bytes =
        build_v2_torrent_bencode(&info_bytes, tracker_urls, creation_date, webseed_urls, &piece_layers_bytes);
    let v2_hash_hex = hex::encode(v2_hash);
    let magnet_uri = build_v2_magnet_uri(&v2_hash_hex, &name, tracker_urls);
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
        tracker_urls: tracker_urls.to_vec(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn build_hybrid(
    tracker_urls: &[String],
    webseed_urls: &[String],
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
    let torrent_bytes =
        build_v2_torrent_bencode(&info_bytes, tracker_urls, creation_date, webseed_urls, &piece_layers_bytes);
    let v1_hex = hex::encode(info_hash);
    let v2_hex = hex::encode(v2_hash);
    let magnet_uri = build_hybrid_magnet_uri(&v1_hex, &v2_hex, &name, tracker_urls);
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
        tracker_urls: tracker_urls.to_vec(),
    })
}

pub fn torrent_creation_date() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}