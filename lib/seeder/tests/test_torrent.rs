use seeder::torrent::structs::file_entry::FileEntry;
use seeder::torrent::torrent::{
    build_hybrid_magnet_uri,
    build_info_bencode,
    build_magnet_uri,
    build_magnet_uri_simple,
    build_piece_layers_bencode,
    build_torrent_bencode,
    build_v2_file_tree_bencode,
    build_v2_info_bencode,
    build_v2_magnet_uri,
    collect_dir_files,
    hash_pieces,
    parse_magnet,
    parse_torrent_meta,
    sha256_block,
    torrent_creation_date,
    write_bencode_string
};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn temp_file_with(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content).unwrap();
    path
}

fn str_trackers(urls: &[&str]) -> Vec<String> {
    urls.iter().map(|s| s.to_string()).collect()
}

#[test]
fn bencode_string_empty() {
    let mut out = Vec::new();
    write_bencode_string(&mut out, b"");
    assert_eq!(out, b"0:");
}

#[test]
fn bencode_string_ascii() {
    let mut out = Vec::new();
    write_bencode_string(&mut out, b"hello");
    assert_eq!(out, b"5:hello");
}

#[test]
fn bencode_string_binary() {
    let mut out = Vec::new();
    write_bencode_string(&mut out, &[0x01, 0x02, 0x03]);
    assert_eq!(out, b"3:\x01\x02\x03");
}

#[test]
fn sha256_empty_input() {
    let hash = sha256_block(b"");
    let hex = hex::encode(hash);
    assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn sha256_known_vector() {
    use sha2::{Digest, Sha256};
    let expected: [u8; 32] = Sha256::new().chain_update(b"abc").finalize().into();
    let actual = sha256_block(b"abc");
    assert_eq!(actual, expected);
}

#[test]
fn sha256_different_inputs_differ() {
    assert_ne!(sha256_block(b"foo"), sha256_block(b"bar"));
}

#[test]
fn sha256_deterministic() {
    assert_eq!(sha256_block(b"deterministic"), sha256_block(b"deterministic"));
}

#[test]
fn sha256_returns_32_bytes() {
    let hash = sha256_block(b"test data");
    assert_eq!(hash.len(), 32);
}

#[test]
fn creation_date_nonzero() {
    let ts = torrent_creation_date();
    assert!(ts > 0, "timestamp should be non-zero");
}

#[test]
fn creation_date_reasonable_range() {
    let ts = torrent_creation_date();
    assert!(ts > 1_577_836_800, "timestamp should be after 2020");
}

#[test]
fn parse_magnet_empty_uri() {
    let (trackers, hash, name) = parse_magnet("");
    assert!(trackers.is_empty());
    assert!(hash.is_none());
    assert!(name.is_none());
}

#[test]
fn parse_magnet_no_question_mark() {
    let (trackers, hash, name) = parse_magnet("magnet:notvalid");
    assert!(trackers.is_empty());
    assert!(hash.is_none());
    assert!(name.is_none());
}

#[test]
fn parse_magnet_basic() {
    let uri = "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd\
               &dn=TestFile\
               &tr=http%3A%2F%2Ftracker.example.com%2Fannounce";
    let (trackers, hash, name) = parse_magnet(uri);
    assert_eq!(name.as_deref(), Some("TestFile"));
    assert!(hash.is_some());
    assert_eq!(hex::encode(hash.unwrap()), "aabbccddeeff00112233445566778899aabbccdd");
    assert_eq!(trackers.len(), 1);
    assert_eq!(trackers[0], "http://tracker.example.com/announce");
}

#[test]
fn parse_magnet_multiple_trackers() {
    let uri = "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd\
               &tr=http://tracker1.com/announce\
               &tr=http://tracker2.com/announce";
    let (trackers, _, _) = parse_magnet(uri);
    assert_eq!(trackers.len(), 2);
}

#[test]
fn parse_magnet_invalid_hash_ignored() {
    let uri = "magnet:?xt=urn:btih:aabbccddeeff001122334455667788&dn=Test";
    let (_, hash, _) = parse_magnet(uri);
    assert!(hash.is_none(), "short/invalid hash should be None");
}

#[test]
fn parse_magnet_duplicate_trackers_deduplicated() {
    let uri = "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd\
               &tr=http://tracker.com/announce\
               &tr=http://tracker.com/announce";
    let (trackers, _, _) = parse_magnet(uri);
    assert_eq!(trackers.len(), 1);
}

#[test]
fn build_magnet_uri_no_trackers() {
    let uri = build_magnet_uri("deadbeef", "MyFile", &[]);
    assert!(uri.starts_with("magnet:?xt=urn:btih:deadbeef&dn="));
    assert!(!uri.contains("&tr="));
}

#[test]
fn build_magnet_uri_with_tracker() {
    let trackers = str_trackers(&["http://tracker.example.com/announce"]);
    let uri = build_magnet_uri("deadbeef", "My File", &trackers);
    assert!(uri.contains("&tr="));
    assert!(uri.contains("deadbeef"));
}

#[test]
fn build_magnet_uri_simple_no_trackers() {
    let uri = build_magnet_uri_simple("deadbeef", "MyFile", &[]);
    assert!(uri.starts_with("magnet:?xt=urn:btih:deadbeef&dn="));
}

#[test]
fn build_v2_magnet_uri_contains_btmh() {
    let uri = build_v2_magnet_uri("cafebabe", "Test", &[]);
    assert!(uri.contains("urn:btmh:1220cafebabe"));
}

#[test]
fn build_hybrid_magnet_uri_contains_both() {
    let uri = build_hybrid_magnet_uri("aabbccdd", "cafebabe", "Test", &[]);
    assert!(uri.contains("urn:btih:aabbccdd"));
    assert!(uri.contains("urn:btmh:1220cafebabe"));
}

#[test]
fn build_info_bencode_single_file() {
    let dir = TempDir::new().unwrap();
    let fp = temp_file_with(&dir, "test.txt", b"hello");
    let files = vec![FileEntry {
        path: fp,
        name: vec!["test.txt".to_string()],
        length: 5,
        offset: 0,
    }];
    let bytes = build_info_bencode("test.txt", 16384, &[0u8; 20], &files, 5);
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("6:length"), "single-file should have length key");
    assert!(!s.contains("5:files"), "single-file should not have files list");
    assert!(s.contains("4:name"), "should contain name key");
    assert!(s.contains("12:piece length"), "should contain piece length");
}

#[test]
fn build_info_bencode_multi_file() {
    let dir = TempDir::new().unwrap();
    let f1 = temp_file_with(&dir, "a.txt", b"aaa");
    let f2 = temp_file_with(&dir, "b.txt", b"bbb");
    let files = vec![
        FileEntry { path: f1, name: vec!["a.txt".to_string()], length: 3, offset: 0 },
        FileEntry { path: f2, name: vec!["b.txt".to_string()], length: 3, offset: 3 },
    ];
    let bytes = build_info_bencode("album", 16384, &[0u8; 40], &files, 6);
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("5:files"), "multi-file should have files key");
    assert!(bytes.starts_with(b"d5:files"), "multi-file info dict must start with files key, not a top-level length");
}

#[test]
fn build_torrent_bencode_contains_info() {
    let info = b"d4:infod4:test5:valuee";
    let trackers = str_trackers(&["http://t.example.com/announce"]);
    let bytes = build_torrent_bencode(info, &trackers, 1700000000, &[]);
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("8:announce"), "should have announce key");
    assert!(s.contains("13:creation date"), "should have creation date");
    assert!(s.contains("10:created by"), "should have created by");
    assert!(s.contains("4:info"), "should have info key");
}

#[test]
fn build_torrent_bencode_no_tracker() {
    let info = b"d4:test5:valuee";
    let bytes = build_torrent_bencode(info, &[], 0, &[]);
    let s = String::from_utf8_lossy(&bytes);
    assert!(!s.contains("13:announce-list"));
}

#[test]
fn build_torrent_bencode_with_webseed() {
    let info = b"d4:test5:valuee";
    let trackers = str_trackers(&["http://t.example.com/announce"]);
    let webseeds = str_trackers(&["http://webseed.example.com/file.mkv"]);
    let bytes = build_torrent_bencode(info, &trackers, 0, &webseeds);
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("8:url-list"), "should embed webseed url-list");
}

#[test]
fn build_v2_file_tree_single() {
    let dir = TempDir::new().unwrap();
    let fp = temp_file_with(&dir, "file.txt", b"data");
    let files = vec![FileEntry {
        path: fp,
        name: vec!["file.txt".to_string()],
        length: 4,
        offset: 0,
    }];
    let roots = [[0u8; 32]];
    let bytes = build_v2_file_tree_bencode(&files, &roots);
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("file.txt"), "file tree should contain file name");
    assert!(s.starts_with('d'), "file tree must be a bencode dict");
    assert!(s.ends_with('e'), "file tree must end with 'e'");
}

#[test]
fn build_v2_info_bencode_contains_required_keys() {
    let dir = TempDir::new().unwrap();
    let fp = temp_file_with(&dir, "file.dat", b"hello world");
    let files = vec![FileEntry {
        path: fp,
        name: vec!["file.dat".to_string()],
        length: 11,
        offset: 0,
    }];
    let roots = [[1u8; 32]];
    let bytes = build_v2_info_bencode("TestTorrent", 16384, &files, &roots);
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("9:file tree"), "v2 info must contain file tree");
    assert!(s.contains("12:meta version"), "v2 info must contain meta version");
    assert!(s.contains("4:name"), "v2 info must contain name");
    assert!(s.contains("12:piece length"), "v2 info must contain piece length");
}

#[test]
fn build_piece_layers_empty() {
    let bytes = build_piece_layers_bencode(&[]);
    assert_eq!(bytes, b"de");
}

#[test]
fn build_piece_layers_sorted_by_root() {
    let root_b = [0xbbu8; 32];
    let root_a = [0xaau8; 32];
    let layer_a = vec![[0x01u8; 32]];
    let layer_b = vec![[0x02u8; 32]];
    let bytes = build_piece_layers_bencode(&[(root_b, layer_b), (root_a, layer_a.clone())]);
    let pos_a = bytes.windows(32).position(|w| w == &[0xaau8; 32][..]).unwrap();
    let pos_b = bytes.windows(32).position(|w| w == &[0xbbu8; 32][..]).unwrap();
    assert!(pos_a < pos_b, "roots should be sorted ascending");
}

#[test]
fn hash_pieces_single_file() {
    let dir = TempDir::new().unwrap();
    let content = b"Hello, World! This is piece data.";
    let fp = temp_file_with(&dir, "data.bin", content);
    let files = vec![FileEntry {
        path: fp,
        name: vec!["data.bin".to_string()],
        length: content.len() as u64,
        offset: 0,
    }];
    let piece_length = 1024u64;
    let total_size = content.len() as u64;
    let piece_count = 1usize;
    let result = hash_pieces(&files, piece_length, total_size, piece_count).unwrap();
    assert_eq!(result.len(), 20);
}

#[test]
fn hash_pieces_deterministic() {
    let dir = TempDir::new().unwrap();
    let content = b"reproducible content";
    let fp = temp_file_with(&dir, "data.bin", content);
    let fp2 = {
        let dir2 = TempDir::new().unwrap();
        let p = temp_file_with(&dir2, "data.bin", content);
        std::mem::forget(dir2);
        p
    };
    let make_files = |path: PathBuf| vec![FileEntry {
        length: content.len() as u64,
        path,
        name: vec!["data.bin".to_string()],
        offset: 0,
    }];
    let h1 = hash_pieces(&make_files(fp), 1024, content.len() as u64, 1).unwrap();
    let h2 = hash_pieces(&make_files(fp2), 1024, content.len() as u64, 1).unwrap();
    assert_eq!(h1, h2, "same content should produce same hash");
}

#[test]
fn hash_pieces_two_pieces() {
    let dir = TempDir::new().unwrap();
    let content = [0u8; 7];
    let fp = temp_file_with(&dir, "data.bin", &content);
    let files = vec![FileEntry {
        path: fp,
        name: vec!["data.bin".to_string()],
        length: 7,
        offset: 0,
    }];
    let result = hash_pieces(&files, 3, 7, 3).unwrap();
    assert_eq!(result.len(), 60);
}

#[test]
fn collect_dir_files_flat() {
    let dir = TempDir::new().unwrap();
    temp_file_with(&dir, "a.txt", b"a");
    temp_file_with(&dir, "b.txt", b"b");
    let mut out = Vec::new();
    collect_dir_files(dir.path(), dir.path(), &mut out).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].1, vec!["a.txt".to_string()]);
    assert_eq!(out[1].1, vec!["b.txt".to_string()]);
}

#[test]
fn collect_dir_files_nested() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    temp_file_with(&dir, "root.txt", b"r");
    {
        let path = sub.join("nested.txt");
        std::fs::write(&path, b"n").unwrap();
    }
    let mut out = Vec::new();
    collect_dir_files(dir.path(), dir.path(), &mut out).unwrap();
    assert_eq!(out.len(), 2);
    let found_nested = out.iter().any(|(_, names)| names == &["sub".to_string(), "nested.txt".to_string()]);
    assert!(found_nested, "nested file should have relative path components");
}

#[test]
fn collect_dir_files_empty_dir() {
    let dir = TempDir::new().unwrap();
    let mut out = Vec::new();
    collect_dir_files(dir.path(), dir.path(), &mut out).unwrap();
    assert!(out.is_empty());
}

fn make_simple_torrent(tracker: &str, name: &str, content: &[u8]) -> Vec<u8> {
    use seeder::torrent::torrent::{
        build_info_bencode, build_torrent_bencode, hash_pieces, torrent_creation_date,
    };
    use std::io::Write;
    let dir = TempDir::new().unwrap();
    let fp = {
        let path = dir.path().join("f");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    };
    let files = vec![FileEntry {
        path: fp,
        name: vec![name.to_string()],
        length: content.len() as u64,
        offset: 0,
    }];
    let pl = 16384u64;
    let pc = 1usize;
    let pieces = hash_pieces(&files, pl, content.len() as u64, pc).unwrap();
    let info = build_info_bencode(name, pl, &pieces, &files, content.len() as u64);
    let trackers = vec![tracker.to_string()];
    build_torrent_bencode(&info, &trackers, torrent_creation_date(), &[])
}

#[test]
fn parse_torrent_meta_basic() {
    let data = make_simple_torrent("http://tracker.example.com/announce", "test.dat", b"hello");
    let meta = parse_torrent_meta(&data).unwrap();
    assert_eq!(meta.name, "test.dat");
    assert_eq!(meta.total_size, 5);
    assert!(!meta.info_hash.iter().all(|&b| b == 0), "info hash should not be all zeros");
    assert!(meta.tracker_urls.contains(&"http://tracker.example.com/announce".to_string()));
}

#[test]
fn parse_torrent_meta_info_hash_stable() {
    let data = make_simple_torrent("http://t.example.com/announce", "file.bin", b"deterministic");
    let m1 = parse_torrent_meta(&data).unwrap();
    let m2 = parse_torrent_meta(&data).unwrap();
    assert_eq!(m1.info_hash, m2.info_hash, "same data → same info hash");
}

#[test]
fn parse_torrent_meta_empty_input_errors() {
    let result = parse_torrent_meta(b"");
    assert!(result.is_err());
}

#[test]
fn parse_torrent_meta_invalid_bencode_errors() {
    let result = parse_torrent_meta(b"not bencode at all");
    assert!(result.is_err());
}