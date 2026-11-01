mod common;

use std::sync::Arc;
use std::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr
};
use torrust_actix::udp::enums::request::Request;
use torrust_actix::udp::enums::response::Response;
use torrust_actix::udp::structs::transaction_id::TransactionId;
use torrust_actix::udp::udp::PROTOCOL_IDENTIFIER;

#[test]
fn test_udp_connect_request_parsing() {
    let mut packet = vec![];
    packet.extend_from_slice(&PROTOCOL_IDENTIFIER.to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&12345u32.to_be_bytes());
    let result = Request::from_bytes(&packet, 74);
    assert!(result.is_ok(), "Should parse valid connect request");
    match result.unwrap() {
        Request::Connect(connect_req) => {
            assert_eq!(connect_req.transaction_id.0, 12345, "Transaction ID should match");
        }
        _ => panic!("Should be Connect request"),
    }
}

#[test]
fn test_udp_malformed_packet() {
    let packet = vec![1, 2, 3];
    let result = Request::from_bytes(&packet, 74);
    assert!(result.is_err(), "Should fail on malformed packet");
}

#[test]
fn test_udp_connect_response_writing() {
    use torrust_actix::udp::structs::connect_response::ConnectResponse;
    use torrust_actix::udp::structs::connection_id::ConnectionId;

    let response = ConnectResponse {
        transaction_id: TransactionId(12345),
        connection_id: ConnectionId(67890),
    };
    let mut buffer = Vec::new();
    let result = Response::Connect(response).write(&mut buffer);
    assert!(result.is_ok(), "Should write connect response successfully");
    assert!(!buffer.is_empty(), "Buffer should contain data");
    assert_eq!(buffer.len(), 16, "Connect response should be 16 bytes");
}

#[test]
fn test_udp_zero_copy_optimization() {
    let packet_data = [0u8; 1496];
    let _data_slice = &packet_data[0..16];
    let mut packet = vec![];
    packet.extend_from_slice(&PROTOCOL_IDENTIFIER.to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&12345u32.to_be_bytes());
    let result = Request::from_bytes(&packet[..], 74);
    assert!(result.is_ok(), "Should parse from slice without Vec allocation");
}

#[tokio::test]
async fn test_udp_announce_request_parsing() {
    let mut packet = vec![];
    packet.extend_from_slice(&12345u64.to_be_bytes());
    packet.extend_from_slice(&1u32.to_be_bytes());
    packet.extend_from_slice(&54321u32.to_be_bytes());
    packet.extend_from_slice(&[0u8; 20]);
    packet.extend_from_slice(&[1u8; 20]);
    packet.extend_from_slice(&0u64.to_be_bytes());
    packet.extend_from_slice(&1000u64.to_be_bytes());
    packet.extend_from_slice(&0u64.to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&(-1i32).to_be_bytes());
    packet.extend_from_slice(&6881u16.to_be_bytes());
    let result = Request::from_bytes(&packet, 74);
    assert!(result.is_ok(), "Should parse valid announce request");
    match result.unwrap() {
        Request::Announce(announce_req) => {
            assert_eq!(announce_req.transaction_id.0, 54321);
            assert_eq!(announce_req.port.0, 6881);
        }
        _ => panic!("Should be Announce request"),
    }
}

#[tokio::test]
async fn test_udp_scrape_request_parsing() {
    let mut packet = vec![];
    packet.extend_from_slice(&12345u64.to_be_bytes());
    packet.extend_from_slice(&2u32.to_be_bytes());
    packet.extend_from_slice(&99999u32.to_be_bytes());
    packet.extend_from_slice(&[0u8; 20]);
    let result = Request::from_bytes(&packet, 74);
    assert!(result.is_ok(), "Should parse valid scrape request");
    match result.unwrap() {
        Request::Scrape(scrape_req) => {
            assert_eq!(scrape_req.transaction_id.0, 99999);
            assert_eq!(scrape_req.info_hashes.len(), 1);
        }
        _ => panic!("Should be Scrape request"),
    }
}

#[tokio::test]
async fn test_udp_packet_size_limits() {
    let mut packet = vec![];
    packet.extend_from_slice(&12345u64.to_be_bytes());
    packet.extend_from_slice(&2u32.to_be_bytes());
    packet.extend_from_slice(&1u32.to_be_bytes());
    for _ in 0..80 {
        packet.extend_from_slice(&[0u8; 20]);
    }
    let result = Request::from_bytes(&packet, 74);
    assert!(result.is_ok(), "Should handle excessive scrape requests gracefully");
}

#[test]
fn test_response_estimated_size() {
    use torrust_actix::udp::structs::connect_response::ConnectResponse;
    use torrust_actix::udp::structs::connection_id::ConnectionId;

    let response = Response::Connect(ConnectResponse {
        transaction_id: TransactionId(1),
        connection_id: ConnectionId(2),
    });
    let estimated = response.estimated_size();
    assert!(estimated > 0, "Should estimate response size");
    assert_eq!(estimated, 16, "Connect response size should be 16 bytes");
}

#[tokio::test]
async fn test_connection_id_is_stable_per_client_within_window() {
    use torrust_actix::udp::structs::udp_server::UdpServer;

    // An id must be reproducible per client, or the tracker cannot verify the one echoed back.
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    let conn_id1 = UdpServer::get_connection_id(&addr).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let conn_id2 = UdpServer::get_connection_id(&addr).await;
    assert_eq!(conn_id1.0, conn_id2.0, "Connection ID should be stable within a time window");
}

#[tokio::test]
async fn test_connection_id_differs_per_client() {
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    let other_port = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6882);
    let other_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 6881);
    let base = UdpServer::get_connection_id(&addr).await;
    assert_ne!(base.0, UdpServer::get_connection_id(&other_port).await.0);
    assert_ne!(base.0, UdpServer::get_connection_id(&other_ip).await.0);
}

#[tokio::test]
async fn test_connection_id_validation_rejects_forged_ids() {
    use torrust_actix::udp::structs::connection_id::ConnectionId;
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)), 6881);
    let issued = UdpServer::get_connection_id(&addr).await;
    assert!(UdpServer::connection_id_valid(&addr, issued), "an id we issued must validate");
    assert!(!UdpServer::connection_id_valid(&addr, ConnectionId(0)), "a guessed id must not validate");
    assert!(
        !UdpServer::connection_id_valid(&addr, ConnectionId(issued.0 ^ 1)),
        "a tampered id must not validate"
    );

    // Ids are bound to their address, which is what stops spoofed announces being answered.
    let victim = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9)), 6881);
    assert!(
        !UdpServer::connection_id_valid(&victim, issued),
        "an id issued to one address must not validate for another"
    );
}

#[test]
fn test_protocol_identifier_constant() {
    assert_eq!(PROTOCOL_IDENTIFIER, 0x41727101980, "Protocol ID should match BEP 15 spec");
}
/// Builds a BEP 15 announce datagram, optionally with a trailing option-2 (path) field.
fn build_announce_packet(connection_id: i64, path: Option<&[u8]>) -> Vec<u8> {
    let mut packet = Vec::with_capacity(120);
    packet.extend_from_slice(&connection_id.to_be_bytes());
    packet.extend_from_slice(&1i32.to_be_bytes()); // action: announce
    packet.extend_from_slice(&7777i32.to_be_bytes()); // transaction id
    packet.extend_from_slice(&[0xAAu8; 20]); // info_hash
    packet.extend_from_slice(&[0xBBu8; 20]); // peer_id
    packet.extend_from_slice(&0i64.to_be_bytes()); // downloaded
    packet.extend_from_slice(&100i64.to_be_bytes()); // left
    packet.extend_from_slice(&0i64.to_be_bytes()); // uploaded
    packet.extend_from_slice(&0i32.to_be_bytes()); // event
    packet.extend_from_slice(&[0u8; 4]); // ip
    packet.extend_from_slice(&0u32.to_be_bytes()); // key
    packet.extend_from_slice(&72i32.to_be_bytes()); // peers wanted
    packet.extend_from_slice(&6881u16.to_be_bytes()); // port
    if let Some(path) = path {
        packet.push(2); // option: URL data
        packet.push(path.len() as u8);
        packet.extend_from_slice(path);
    }
    packet
}

#[tokio::test]
async fn test_udp_announce_requires_valid_connection_id() {
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    use torrust_actix::udp::enums::response::Response;
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let config = Arc::new(common::build_test_config(|_| {}));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)), 6881);

    // An id the tracker never issued must be refused, or it acts as a UDP reflector.
    let forged = build_announce_packet(0x0123_4567_89ab_cdef, None);
    let response = UdpServer::handle_packet(addr, &forged, tracker.clone(), false).await.unwrap();
    assert!(matches!(response, Response::Error(_)), "forged connection id must be rejected");

    let issued = UdpServer::get_connection_id(&addr).await;
    let valid = build_announce_packet(issued.0, None);
    let response = UdpServer::handle_packet(addr, &valid, tracker.clone(), false).await.unwrap();
    assert!(matches!(response, Response::AnnounceIpv4(_)), "issued connection id must be accepted, got {response:?}");
}

#[tokio::test]
async fn test_udp_announce_path_slicing_does_not_panic() {
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    use torrust_actix::udp::structs::udp_server::UdpServer;

    // A path whose bytes straddle the key offsets must not panic: under panic = 'abort' one
    // datagram would take the whole tracker down.
    let config = Arc::new(common::build_test_config(|config| {
        config.tracker_config.keys_enabled = true;
        config.tracker_config.users_enabled = true;
    }));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 11)), 6881);
    let issued = UdpServer::get_connection_id(&addr).await;

    let mut paths: Vec<Vec<u8>> = Vec::new();
    // A multi-byte character crossing byte offset 50 (the end of the key field).
    let mut multibyte = b"a".repeat(49);
    multibyte.extend_from_slice("\u{00e9}".as_bytes());
    paths.push(multibyte);
    // A multi-byte character crossing byte offset 91 (the end of the user-key field).
    let mut multibyte_user = b"b".repeat(90);
    multibyte_user.extend_from_slice("\u{00e9}".as_bytes());
    paths.push(multibyte_user);
    // Lengths that land exactly on a field boundary.
    paths.push(b"c".repeat(50));
    paths.push(b"d".repeat(91));
    paths.push(Vec::new());
    paths.push(b"/announce/".to_vec());

    for path in paths {
        let packet = build_announce_packet(issued.0, Some(&path));
        // Any outcome is fine as long as it is a response and not a panic.
        let _ = UdpServer::handle_packet(addr, &packet, tracker.clone(), false).await;
    }
}

#[tokio::test]
async fn test_udp_user_key_path_with_keys_off() {
    use std::collections::BTreeMap;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    use torrust_actix::tracker::structs::user_entry_item::UserEntryItem;
    use torrust_actix::tracker::structs::user_id::UserId;
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let config = Arc::new(common::build_test_config(|config| config.tracker_config.users_enabled = true));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let user_key = UserId([0x12; 20]);
    tracker.add_user(UserId([1; 20]), UserEntryItem {
        key: user_key,
        user_id: Some(1),
        user_uuid: None,
        uploaded: 0,
        downloaded: 0,
        completed: 0,
        updated: 0,
        active: 1,
        torrents_active: BTreeMap::new(),
    });
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 12)), 6881);
    let issued = UdpServer::get_connection_id(&addr).await;
    let key_hex = hex::encode(user_key.0);
    // (path, accepted): `/announce/<user key>` as on HTTP, the old placeholder form, and an unknown key.
    for (path, accepted) in [
        (format!("/announce/{key_hex}"), true),
        (format!("/announce/{}/{key_hex}", "0".repeat(40)), true),
        (format!("/announce/{}", "3".repeat(40)), false),
    ] {
        let packet = build_announce_packet(issued.0, Some(path.as_bytes()));
        let response = UdpServer::handle_packet(addr, &packet, tracker.clone(), false).await.unwrap();
        assert_eq!(matches!(response, Response::AnnounceIpv4(_)), accepted, "{path}: {response:?}");
    }
}

#[tokio::test]
async fn test_udp_replies_only_where_a_reply_belongs() {
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    let tracker = common::create_test_tracker().await;
    // Too short to carry a transaction id: no reply, rather than 19 bytes to a forged source.
    assert!(UdpServer::handle_packet(addr, &[0u8], tracker.clone(), false).await.is_none());
    // An unknown action still names its transaction, so the client gets its error.
    let mut unknown = 0i64.to_be_bytes().to_vec();
    unknown.extend_from_slice(&99i32.to_be_bytes());
    unknown.extend_from_slice(&4242i32.to_be_bytes());
    match UdpServer::handle_packet(addr, &unknown, tracker.clone(), false).await {
        Some(Response::Error(error)) => assert_eq!(error.transaction_id, TransactionId(4242)),
        other => panic!("expected an error reply, got {other:?}"),
    }

    // With keys on, a UDP scrape (which cannot carry one) is refused like an unkeyed HTTP scrape.
    let config = Arc::new(common::build_test_config(|config| config.tracker_config.keys_enabled = true));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let issued = UdpServer::get_connection_id(&addr).await;
    let mut scrape = issued.0.to_be_bytes().to_vec();
    scrape.extend_from_slice(&2i32.to_be_bytes());
    scrape.extend_from_slice(&7i32.to_be_bytes());
    scrape.extend_from_slice(&[0xAAu8; 20]);
    match UdpServer::handle_packet(addr, &scrape, tracker, false).await {
        Some(Response::Error(error)) => assert_eq!(error.message, "unknown key", "the refusal keeps its reason"),
        other => panic!("expected an error reply, got {other:?}"),
    }
}
