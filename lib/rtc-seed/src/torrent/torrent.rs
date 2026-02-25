use crate::torrent::structs::file_entry::FileEntry;
use crate::torrent::types::QUERY_ENCODE;
use percent_encoding::utf8_percent_encode;
use sha1::{
    Digest,
    Sha1
};
use std::fs::File;
use std::io::{
    self,
    Read
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
    let created_by = "Torrust-Actix rtc-seed v0.1";
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