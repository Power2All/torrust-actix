use seeder::config::enums::seed_protocol::SeedProtocol;
use seeder::seeder::seeder::generate_peer_id;

#[test]
fn bt_has_bt() {
    assert!(SeedProtocol::Bt.has_bt());
}

#[test]
fn bt_no_rtc() {
    assert!(!SeedProtocol::Bt.has_rtc());
}

#[test]
fn rtc_no_bt() {
    assert!(!SeedProtocol::Rtc.has_bt());
}

#[test]
fn rtc_has_rtc() {
    assert!(SeedProtocol::Rtc.has_rtc());
}

#[test]
fn both_has_bt() {
    assert!(SeedProtocol::Both.has_bt());
}

#[test]
fn both_has_rtc() {
    assert!(SeedProtocol::Both.has_rtc());
}

#[test]
fn default_is_both() {
    assert_eq!(SeedProtocol::default(), SeedProtocol::Both);
}

#[test]
fn serde_json_bt() {
    let j = serde_json::to_string(&SeedProtocol::Bt).unwrap();
    assert_eq!(j, "\"bt\"");
    let v: SeedProtocol = serde_json::from_str("\"bt\"").unwrap();
    assert_eq!(v, SeedProtocol::Bt);
}

#[test]
fn serde_json_rtc() {
    let j = serde_json::to_string(&SeedProtocol::Rtc).unwrap();
    assert_eq!(j, "\"rtc\"");
    let v: SeedProtocol = serde_json::from_str("\"rtc\"").unwrap();
    assert_eq!(v, SeedProtocol::Rtc);
}

#[test]
fn serde_json_both() {
    let j = serde_json::to_string(&SeedProtocol::Both).unwrap();
    assert_eq!(j, "\"both\"");
    let v: SeedProtocol = serde_json::from_str("\"both\"").unwrap();
    assert_eq!(v, SeedProtocol::Both);
}

#[test]
fn serde_yaml_round_trip() {
    let yaml = serde_yaml::to_string(&SeedProtocol::Rtc).unwrap();
    let v: SeedProtocol = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(v, SeedProtocol::Rtc);
}

#[test]
fn clone_preserves_variant() {
    assert_eq!(SeedProtocol::Bt.clone(), SeedProtocol::Bt);
    assert_eq!(SeedProtocol::Rtc.clone(), SeedProtocol::Rtc);
    assert_eq!(SeedProtocol::Both.clone(), SeedProtocol::Both);
}

// --- peer_id ---

#[test]
fn peer_id_is_20_bytes() {
    let id = generate_peer_id();
    assert_eq!(id.len(), 20);
}

#[test]
fn peer_id_prefix_is_torrust_seeder() {
    let id = generate_peer_id();
    assert_eq!(&id[..8], b"-TS0420-", "peer ID must carry the Torrust-Seeder fingerprint");
}

#[test]
fn peer_id_suffix_is_ascii_digits() {
    let id = generate_peer_id();
    for &byte in &id[8..] {
        assert!(byte.is_ascii_digit(), "random suffix must be ASCII digits (got 0x{:02x})", byte);
    }
}

#[test]
fn peer_ids_are_unique() {
    let a = generate_peer_id();
    let b = generate_peer_id();
    assert_ne!(a, b, "consecutive peer IDs should differ");
}