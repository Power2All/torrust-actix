#[cfg(test)]
mod tests {
    use crate::config::structs::seeder_config::SeederConfig;
    use crate::torrent::enums::torrent_version::TorrentVersion;
    use crate::torrent::structs::file_entry::FileEntry;
    use crate::torrent::structs::torrent_builder::TorrentBuilder;
    use crate::torrent::torrent::{
        build_hybrid_info_bencode,
        build_hybrid_magnet_uri,
        build_merkle_tree_and_layers,
        build_piece_layers_bencode,
        build_v2_file_tree_bencode,
        build_v2_info_bencode,
        build_v2_magnet_uri,
        build_v2_torrent_bencode,
        sha256_block,
        write_bencode_string
    };
    use std::path::PathBuf;

    fn write_tmp(prefix: &str, data: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("rtc_test_{}_{}.bin", prefix, std::process::id()));
        std::fs::write(&path, data).expect("write tmp");
        path
    }

    fn cleanup(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sha256_empty_string() {
        let h = sha256_block(b"");
        let expected = hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
            .unwrap();
        assert_eq!(h.as_ref(), expected.as_slice());
    }

    #[test]
    fn sha256_abc() {
        let h = sha256_block(b"abc");
        let hex_str = hex::encode(h);
        assert_eq!(hex_str.len(), 64, "SHA-256 output must be 64 hex chars (32 bytes)");
        assert_eq!(
            hex_str,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn merkle_empty_file() {
        let path = write_tmp("empty", b"");
        let (root, layers) = build_merkle_tree_and_layers(&path, 0, 16 * 1024).unwrap();
        cleanup(&path);
        assert_eq!(root, [0u8; 32], "empty file root must be all-zero");
        assert!(layers.is_empty(), "empty file has no piece layers");
    }

    #[test]
    fn merkle_single_block_file() {
        let data = vec![0x42u8; 100];
        let path = write_tmp("single_block", &data);
        let piece_length = 16 * 1024u64;
        let (root, layers) = build_merkle_tree_and_layers(&path, data.len() as u64, piece_length).unwrap();
        cleanup(&path);
        let mut block = vec![0u8; 16384];
        block[..100].fill(0x42);
        let expected_leaf = sha256_block(&block);
        assert_eq!(root, expected_leaf, "root of 1-block file == leaf hash");
        assert!(layers.is_empty(), "single-block file ≤ piece_length: no layer hashes");
    }

    #[test]
    fn merkle_two_block_file_piece_length_equals_block_size() {
        const BLOCK: u64 = 16 * 1024;
        let data_a = vec![0xAAu8; BLOCK as usize];
        let data_b = vec![0xBBu8; BLOCK as usize];
        let mut data = data_a.clone();
        data.extend_from_slice(&data_b);
        let path = write_tmp("two_blocks", &data);
        let file_size = data.len() as u64;
        let piece_length = BLOCK;
        let (root, layers) = build_merkle_tree_and_layers(&path, file_size, piece_length).unwrap();
        cleanup(&path);
        let leaf_a = sha256_block(&data_a);
        let leaf_b = sha256_block(&data_b);
        let mut pair = [0u8; 64];
        pair[..32].copy_from_slice(&leaf_a);
        pair[32..].copy_from_slice(&leaf_b);
        let expected_root = sha256_block(&pair);
        assert_eq!(root, expected_root);
        assert_eq!(layers.len(), 2, "two pieces → two piece layer hashes");
        assert_eq!(layers[0], leaf_a);
        assert_eq!(layers[1], leaf_b);
    }

    #[test]
    fn merkle_multi_piece_file() {
        const BLOCK: u64 = 16 * 1024;
        let piece_length = 2 * BLOCK;
        let data = vec![0x77u8; 3 * BLOCK as usize];
        let path = write_tmp("multi_piece", &data);
        let file_size = data.len() as u64;
        let (root, layers) = build_merkle_tree_and_layers(&path, file_size, piece_length).unwrap();
        cleanup(&path);
        let leaf_block = vec![0x77u8; BLOCK as usize];
        let leaf = sha256_block(&leaf_block);
        let pad = [0u8; 32];
        let mut pair = [0u8; 64];
        pair[..32].copy_from_slice(&leaf);
        pair[32..].copy_from_slice(&leaf);
        let piece0_hash = sha256_block(&pair);
        pair[..32].copy_from_slice(&leaf);
        pair[32..].copy_from_slice(&pad);
        let piece1_hash = sha256_block(&pair);
        pair[..32].copy_from_slice(&piece0_hash);
        pair[32..].copy_from_slice(&piece1_hash);
        let expected_root = sha256_block(&pair);
        assert_eq!(root, expected_root);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0], piece0_hash);
        assert_eq!(layers[1], piece1_hash);
    }

    fn make_file_entry(name: &str, length: u64, offset: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(name),
            name: vec![name.to_string()],
            length,
            offset,
        }
    }

    #[test]
    fn v2_file_tree_single_file() {
        let root = [0xABu8; 32];
        let files = vec![make_file_entry("movie.mp4", 1000, 0)];
        let roots = vec![root];
        let bencode = build_v2_file_tree_bencode(&files, &roots);
        let s = String::from_utf8_lossy(&bencode);
        assert!(s.starts_with('d'), "file tree must be a bencode dict");
        assert!(s.contains("movie.mp4"), "file tree must contain filename");
        assert!(s.contains("6:length"), "file tree must contain length key");
        assert!(s.contains("11:pieces root"), "file tree must contain pieces root key");
        let files_empty = vec![make_file_entry("empty.bin", 0, 0)];
        let roots_empty = vec![[0u8; 32]];
        let b2 = build_v2_file_tree_bencode(&files_empty, &roots_empty);
        let s2 = String::from_utf8_lossy(&b2);
        assert!(!s2.contains("pieces root"), "empty file must omit pieces root");
    }

    #[test]
    fn v2_file_tree_multi_file_sorted() {
        let files = vec![
            make_file_entry("zebra.txt", 100, 0),
            make_file_entry("apple.txt", 200, 100),
        ];
        let roots = vec![[1u8; 32], [2u8; 32]];
        let bencode = build_v2_file_tree_bencode(&files, &roots);
        let s = String::from_utf8_lossy(&bencode);
        let pos_apple = s.find("apple.txt").expect("apple.txt must be present");
        let pos_zebra = s.find("zebra.txt").expect("zebra.txt must be present");
        assert!(pos_apple < pos_zebra, "file tree keys must be sorted: 'apple' before 'zebra'");
    }

    #[test]
    fn piece_layers_empty() {
        let layers: Vec<([u8; 32], Vec<[u8; 32]>)> = vec![];
        let b = build_piece_layers_bencode(&layers);
        assert_eq!(b, b"de", "empty piece layers must be 'de'");
    }

    #[test]
    fn piece_layers_single_entry() {
        let root = [0x11u8; 32];
        let hash1 = [0x22u8; 32];
        let hash2 = [0x33u8; 32];
        let layers = vec![(root, vec![hash1, hash2])];
        let b = build_piece_layers_bencode(&layers);
        assert!(b.starts_with(b"d"), "must start with 'd'");
        assert!(b.ends_with(b"e"), "must end with 'e'");
        let key_prefix = b"32:";
        let key_start = b"d".len();
        assert_eq!(&b[key_start..key_start + 3], key_prefix);
        assert_eq!(&b[key_start + 3..key_start + 35], &root);
        let val_start = key_start + 35;
        let expected_val_prefix = b"64:";
        assert_eq!(&b[val_start..val_start + 3], expected_val_prefix);
        assert_eq!(&b[val_start + 3..val_start + 35], &hash1);
        assert_eq!(&b[val_start + 35..val_start + 67], &hash2);
    }

    #[test]
    fn piece_layers_sorted_by_root() {
        let root_b = [0xBBu8; 32];
        let root_a = [0xAAu8; 32];
        let layers = vec![
            (root_b, vec![[0x01u8; 32]]),
            (root_a, vec![[0x02u8; 32]]),
        ];
        let b = build_piece_layers_bencode(&layers);
        let pos_aa = b.windows(32).position(|w| w == &root_a).expect("root_a must be present");
        let pos_bb = b.windows(32).position(|w| w == &root_b).expect("root_b must be present");
        assert!(pos_aa < pos_bb, "piece layers must be sorted by root bytes");
    }

    #[test]
    fn v2_info_key_order() {
        let files = vec![make_file_entry("test.mp4", 500, 0)];
        let roots = vec![[0xCCu8; 32]];
        let b = build_v2_info_bencode("test", 16384, &files, &roots);
        let s = String::from_utf8_lossy(&b);
        let p_ft = s.find("file tree").expect("file tree must be present");
        let p_mv = s.find("meta version").expect("meta version must be present");
        let p_na = s.find("name").expect("name must be present");
        let p_pl = s.find("piece length").expect("piece length must be present");
        assert!(p_ft < p_mv, "file tree < meta version");
        assert!(p_mv < p_na, "meta version < name");
        assert!(p_na < p_pl, "name < piece length");
        assert!(s.contains("i2e"), "meta version must be 2");
        assert!(s.contains("i16384e"), "piece length value must be 16384");
    }

    #[test]
    fn hybrid_info_key_order_single_file() {
        let files = vec![make_file_entry("movie.mp4", 5000, 0)];
        let roots = vec![[0xDDu8; 32]];
        let fake_pieces = vec![0xFFu8; 20];
        let b = build_hybrid_info_bencode("movie", 16384, &fake_pieces, &files, &roots, 5000);
        let s = String::from_utf8_lossy(&b);
        let p_ft  = s.find("file tree").expect("file tree");
        let p_le  = s.find("length").expect("length");
        let p_mv  = s.find("meta version").expect("meta version");
        let p_na  = s.find("name").expect("name");
        let p_pl  = s.find("piece length").expect("piece length");
        let p_pi  = s.find("6:pieces").expect("pieces");
        assert!(p_ft < p_le, "file tree < length");
        assert!(p_le < p_mv, "length < meta version");
        assert!(p_mv < p_na, "meta version < name");
        assert!(p_na < p_pl, "name < piece length");
        assert!(p_pl < p_pi, "piece length < pieces");
        assert!(!s.contains("5:files"), "single-file hybrid must not have 'files' key");
    }

    #[test]
    fn hybrid_info_key_order_multi_file() {
        let files = vec![
            make_file_entry("a.mp4", 3000, 0),
            make_file_entry("b.mp4", 2000, 3000),
        ];
        let roots = vec![[0x01u8; 32], [0x02u8; 32]];
        let fake_pieces = vec![0xFFu8; 40];
        let b = build_hybrid_info_bencode("multi", 16384, &fake_pieces, &files, &roots, 5000);
        let s = String::from_utf8_lossy(&b);
        let p_ft = s.find("file tree").expect("file tree");
        let p_fi = s.find("5:files").expect("files list");
        let p_mv = s.find("meta version").expect("meta version");
        let p_pl = s.find("piece length").expect("piece length");
        let p_pi = s.find("6:pieces").expect("pieces");
        assert!(p_ft < p_fi, "file tree < files");
        assert!(p_fi < p_mv, "files < meta version");
        assert!(p_mv < p_pl, "meta version < piece length");
        assert!(p_pl < p_pi, "piece length < pieces");
        assert!(s.contains("5:files"), "multi-file hybrid must have 'files' list");
        assert!(!s.contains(&format!("6:lengthi5000e")), "multi-file hybrid must not have top-level length=5000");
    }

    #[test]
    fn v2_torrent_key_order() {
        let info_bytes = b"d4:infoe";
        let piece_layers = b"de";
        let b = build_v2_torrent_bencode(
            info_bytes,
            "http://tracker:6969/announce",
            1_700_000_000,
            &[],
            piece_layers,
        );
        let s = String::from_utf8_lossy(&b);
        let p_ann = s.find("announce").expect("announce");
        let p_cb  = s.find("created by").expect("created by");
        let p_cd  = s.find("creation date").expect("creation date");
        let p_inf = s.find("4:info").expect("info");
        let p_pl  = s.find("piece layers").expect("piece layers");
        assert!(p_ann < p_cb, "announce < created by");
        assert!(p_cb  < p_cd, "created by < creation date");
        assert!(p_cd  < p_inf, "creation date < info");
        assert!(p_inf < p_pl, "info < piece layers");
    }

    #[test]
    fn v2_torrent_with_webseed() {
        let info_bytes = b"de";
        let piece_layers = b"de";
        let b = build_v2_torrent_bencode(
            info_bytes,
            "http://tracker/announce",
            0,
            &["https://cdn.example.com/file.mp4".to_string()],
            piece_layers,
        );
        let s = String::from_utf8_lossy(&b);
        let p_pl = s.find("piece layers").expect("piece layers");
        let p_ul = s.find("url-list").expect("url-list");
        assert!(p_pl < p_ul, "piece layers < url-list");
        assert!(s.contains("cdn.example.com"), "webseed URL must be present");
    }

    #[test]
    fn v2_magnet_uri_format() {
        let hash_hex = "a".repeat(64);
        let uri = build_v2_magnet_uri(&hash_hex, "My Torrent", "http://tracker:6969/announce");
        assert!(
            uri.starts_with("magnet:?xt=urn:btmh:1220"),
            "v2 magnet must use urn:btmh:1220 scheme: {uri}"
        );
        assert!(uri.contains(&hash_hex), "v2 magnet must contain the 64-hex hash");
        assert!(!uri.contains("urn:btih:"), "v2 magnet must not contain urn:btih:");
        assert!(uri.contains("dn="), "magnet must have dn= param");
        assert!(uri.contains("tr="), "magnet must have tr= param");
    }

    #[test]
    fn hybrid_magnet_uri_format() {
        let v1 = "b".repeat(40);
        let v2 = "c".repeat(64);
        let uri = build_hybrid_magnet_uri(&v1, &v2, "Hybrid", "http://tracker/announce");
        assert!(uri.contains("urn:btih:"), "hybrid magnet must contain urn:btih:");
        assert!(uri.contains("urn:btmh:1220"), "hybrid magnet must contain urn:btmh:1220");
        assert!(uri.contains(&v1), "hybrid magnet must contain v1 hash");
        assert!(uri.contains(&v2), "hybrid magnet must contain v2 hash");
    }

    #[test]
    fn bencode_string_encoding() {
        let mut out = Vec::new();
        write_bencode_string(&mut out, b"hello");
        assert_eq!(out, b"5:hello");
        let mut out2 = Vec::new();
        write_bencode_string(&mut out2, b"");
        assert_eq!(out2, b"0:");
    }

    fn default_config(path: PathBuf, version: TorrentVersion) -> SeederConfig {
        SeederConfig {
            tracker_url: "http://127.0.0.1:6969/announce".to_string(),
            file_paths: vec![path],
            name: Some("test_torrent".to_string()),
            out_file: None,
            webseed_urls: vec![],
            ice_servers: vec![],
            rtc_interval_ms: 5000,
            version,
        }
    }

    #[test]
    fn builder_v1_produces_sha1_hash() {
        let data = vec![0x42u8; 1024];
        let path = write_tmp("builder_v1", &data);
        let config = default_config(path.clone(), TorrentVersion::V1);
        let info = TorrentBuilder::build(&config).expect("V1 build must succeed");
        cleanup(&path);
        let hex = hex::encode(info.info_hash);
        assert_eq!(hex.len(), 40, "v1 info_hash must be 40 hex chars");
        assert_eq!(info.version, TorrentVersion::V1);
        assert!(info.v2_info_hash.is_none(), "v1 must not have v2_info_hash");
        assert!(info.magnet_uri.contains("urn:btih:"), "v1 magnet must use urn:btih:");
        assert!(!info.magnet_uri.contains("urn:btmh:"), "v1 magnet must not have urn:btmh:");
        assert!(info.torrent_bytes.starts_with(b"d"), "torrent bytes must start with 'd'");
        let s = String::from_utf8_lossy(&info.torrent_bytes);
        assert!(s.contains("announce"), "v1 torrent must have announce key");
        assert!(s.contains("4:info"), "v1 torrent must have info key");
    }

    #[test]
    fn builder_v2_produces_sha256_hash_and_file_tree() {
        let data = vec![0x55u8; 512];
        let path = write_tmp("builder_v2", &data);
        let config = default_config(path.clone(), TorrentVersion::V2);
        let info = TorrentBuilder::build(&config).expect("V2 build must succeed");
        cleanup(&path);
        assert_eq!(info.version, TorrentVersion::V2);
        assert!(info.v2_info_hash.is_some(), "v2 must have v2_info_hash");
        assert_eq!(hex::encode(info.v2_info_hash.unwrap()).len(), 64, "v2_info_hash must be 64 hex");
        let v2h = info.v2_info_hash.unwrap();
        assert_eq!(info.info_hash, v2h[..20], "v2 info_hash == sha256[..20]");
        assert!(info.magnet_uri.contains("urn:btmh:1220"), "v2 magnet must use urn:btmh:1220");
        assert!(!info.magnet_uri.contains("urn:btih:"), "v2 magnet must not use urn:btih:");
        let s = String::from_utf8_lossy(&info.torrent_bytes);
        assert!(s.contains("piece layers"), "v2 torrent must have piece layers");
        assert!(s.contains("file tree"), "v2 torrent must have file tree");
        assert!(s.contains("meta version"), "v2 torrent must have meta version");
        assert!(!s.contains("6:pieces"), "pure v2 torrent must not have SHA-1 pieces field");
        let info_start = info.torrent_bytes.windows(6)
            .position(|w| w == b"4:info")
            .expect("must find 4:info key");
        let info_dict_bytes = &info.torrent_bytes[info_start + 6..];
        let expected_v2h = sha256_block(info_dict_bytes
            .split(|_| false)
            .next()
            .unwrap_or(info_dict_bytes));
        let _ = expected_v2h;
        let magnet_v2 = hex::encode(v2h);
        assert!(
            info.magnet_uri.contains(&magnet_v2),
            "magnet URI must contain the full 64-hex v2 hash"
        );
    }

    #[test]
    fn builder_hybrid_produces_both_hashes() {
        let data = vec![0x77u8; 2048];
        let path = write_tmp("builder_hybrid", &data);
        let config = default_config(path.clone(), TorrentVersion::Hybrid);
        let info = TorrentBuilder::build(&config).expect("Hybrid build must succeed");
        cleanup(&path);
        assert_eq!(info.version, TorrentVersion::Hybrid);
        assert!(info.v2_info_hash.is_some(), "hybrid must have v2_info_hash");
        let hex_sha1 = hex::encode(info.info_hash);
        assert_eq!(hex_sha1.len(), 40, "hybrid info_hash must be 40-hex SHA-1");
        assert!(info.magnet_uri.contains("urn:btih:"), "hybrid magnet must have urn:btih:");
        assert!(info.magnet_uri.contains("urn:btmh:1220"), "hybrid magnet must have urn:btmh:1220");
        let s = String::from_utf8_lossy(&info.torrent_bytes);
        assert!(s.contains("6:pieces"), "hybrid torrent must have SHA-1 pieces");
        assert!(s.contains("file tree"), "hybrid torrent must have file tree");
        assert!(s.contains("meta version"), "hybrid torrent must have meta version");
        assert!(s.contains("piece layers"), "hybrid torrent must have piece layers");
    }

    #[test]
    fn builder_v1_and_v2_produce_different_torrents() {
        let data = vec![0xA5u8; 1000];
        let path = write_tmp("diff_versions", &data);
        let cfg_v1 = default_config(path.clone(), TorrentVersion::V1);
        let cfg_v2 = default_config(path.clone(), TorrentVersion::V2);
        let info_v1 = TorrentBuilder::build(&cfg_v1).expect("V1");
        let info_v2 = TorrentBuilder::build(&cfg_v2).expect("V2");
        cleanup(&path);
        assert_ne!(
            info_v1.torrent_bytes, info_v2.torrent_bytes,
            "v1 and v2 torrents for the same file must differ"
        );
        assert_ne!(
            hex::encode(info_v1.info_hash),
            hex::encode(info_v2.info_hash),
            "v1 and v2 info hashes must differ"
        );
    }

    #[test]
    fn builder_v1_stability() {
        let data = vec![0x11u8; 512];
        let path = write_tmp("stability_v1", &data);
        let cfg1 = default_config(path.clone(), TorrentVersion::V1);
        let cfg2 = default_config(path.clone(), TorrentVersion::V1);
        let i1 = TorrentBuilder::build(&cfg1).expect("build 1");
        let i2 = TorrentBuilder::build(&cfg2).expect("build 2");
        cleanup(&path);
        assert_eq!(
            i1.info_hash, i2.info_hash,
            "same file + same config → same info_hash"
        );
    }

    #[test]
    fn builder_v2_stability() {
        let data = vec![0x22u8; 512];
        let path = write_tmp("stability_v2", &data);
        let cfg1 = default_config(path.clone(), TorrentVersion::V2);
        let cfg2 = default_config(path.clone(), TorrentVersion::V2);
        let i1 = TorrentBuilder::build(&cfg1).expect("build 1");
        let i2 = TorrentBuilder::build(&cfg2).expect("build 2");
        cleanup(&path);
        assert_eq!(
            i1.v2_info_hash, i2.v2_info_hash,
            "same file + v2 config → same v2_info_hash"
        );
    }
}