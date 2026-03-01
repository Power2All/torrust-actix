use seeder::config::enums::seed_protocol::SeedProtocol;
use seeder::config::structs::torrent_entry::TorrentEntry;
use seeder::torrent::enums::torrent_version::TorrentVersion;

fn default_entry() -> TorrentEntry {
    TorrentEntry {
        out: None,
        name: Some("test".to_string()),
        file: vec!["/tmp/file.dat".to_string()],
        trackers: vec!["http://tracker.example.com/announce".to_string()],
        webseed: None,
        ice: None,
        rtc_interval: None,
        protocol: None,
        version: None,
        torrent_file: None,
        magnet: None,
        enabled: true,
        upload_limit: None,
    }
}

fn default_ice() -> Vec<String> {
    vec!["stun:stun.l.google.com:19302".to_string()]
}

#[test]
fn no_file_and_no_torrent_file_is_error() {
    let mut e = default_entry();
    e.file.clear();
    e.torrent_file = None;
    let result = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000);
    assert!(result.is_err(), "should fail when no files provided");
}

#[test]
fn torrent_file_without_data_files_is_ok() {
    let mut e = default_entry();
    e.file.clear();
    e.torrent_file = Some("/tmp/existing.torrent".to_string());
    let result = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000);
    assert!(result.is_ok(), "torrent_file alone should be sufficient");
}

#[test]
fn inherits_global_protocol_when_none() {
    let e = default_entry();
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Bt, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.protocol, SeedProtocol::Bt);
}

#[test]
fn per_torrent_protocol_overrides_global() {
    let mut e = default_entry();
    e.protocol = Some(SeedProtocol::Rtc);
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.protocol, SeedProtocol::Rtc);
}

#[test]
fn per_torrent_ice_overrides_global() {
    let mut e = default_entry();
    e.ice = Some(vec!["stun:custom.stun.example.com:3478".to_string()]);
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.ice_servers, vec!["stun:custom.stun.example.com:3478".to_string()]);
}

#[test]
fn inherits_global_ice() {
    let e = default_entry();
    let global_ice = vec!["stun:global.example.com:3478".to_string()];
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &global_ice, 5000).unwrap();
    assert_eq!(cfg.ice_servers, global_ice);
}

#[test]
fn falls_back_to_google_stun_when_global_ice_empty() {
    let e = default_entry();
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &[], 5000).unwrap();
    assert!(
        cfg.ice_servers.iter().any(|s| s.contains("stun.l.google.com")),
        "should fall back to Google STUN servers"
    );
    assert_eq!(cfg.ice_servers.len(), 2, "should include both default Google STUN servers");
}

#[test]
fn rtc_interval_seconds_converted_to_ms() {
    let mut e = default_entry();
    e.rtc_interval = Some(3);
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.rtc_interval_ms, 3000, "should convert seconds → ms");
}

#[test]
fn inherits_global_rtc_interval() {
    let e = default_entry();
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 7500).unwrap();
    assert_eq!(cfg.rtc_interval_ms, 7500);
}

#[test]
fn version_default_is_v1() {
    let e = default_entry();
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.version, TorrentVersion::V1);
}

#[test]
fn version_v2() {
    let mut e = default_entry();
    e.version = Some("v2".to_string());
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.version, TorrentVersion::V2);
}

#[test]
fn version_hybrid() {
    let mut e = default_entry();
    e.version = Some("hybrid".to_string());
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.version, TorrentVersion::Hybrid);
}

#[test]
fn version_unknown_defaults_to_v1() {
    let mut e = default_entry();
    e.version = Some("v99".to_string());
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.version, TorrentVersion::V1);
}

#[test]
fn listen_port_forwarded() {
    let e = default_entry();
    let cfg = e.to_seeder_config(None, 51413, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.listen_port, 51413);
}

#[test]
fn upload_limit_forwarded() {
    let mut e = default_entry();
    e.upload_limit = Some(2048);
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.upload_limit, Some(2048));
}

#[test]
fn tracker_urls_forwarded() {
    let mut e = default_entry();
    e.trackers = vec!["http://a.com/announce".to_string(), "http://b.com/announce".to_string()];
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.tracker_urls.len(), 2);
}

#[test]
fn webseed_urls_forwarded() {
    let mut e = default_entry();
    e.webseed = Some(vec!["http://webseed.example.com/file.mkv".to_string()]);
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.webseed_urls.len(), 1);
    assert_eq!(cfg.webseed_urls[0], "http://webseed.example.com/file.mkv");
}

#[test]
fn name_forwarded() {
    let mut e = default_entry();
    e.name = Some("My Movie".to_string());
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert_eq!(cfg.name.as_deref(), Some("My Movie"));
}

#[test]
fn upnp_defaults_false() {
    let e = default_entry();
    let cfg = e.to_seeder_config(None, 6881, SeedProtocol::Both, &default_ice(), 5000).unwrap();
    assert!(!cfg.upnp, "upnp should default to false");
}