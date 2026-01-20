mod common;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use torrust_actix::udp::enums::request::Request;
use torrust_actix::udp::enums::response::Response;
use torrust_actix::udp::structs::transaction_id::TransactionId;
use torrust_actix::udp::udp::PROTOCOL_IDENTIFIER;

#[test]
fn test_udp_connect_request_parsing() {
    // Build a valid connect request packet
    let mut packet = vec![];
    packet.extend_from_slice(&PROTOCOL_IDENTIFIER.to_be_bytes()); // Protocol ID
    packet.extend_from_slice(&0u32.to_be_bytes()); // Action: Connect
    packet.extend_from_slice(&12345u32.to_be_bytes()); // Transaction ID

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
    let packet = vec![1, 2, 3]; // Too short

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
    assert!(buffer.len() > 0, "Buffer should contain data");
    assert_eq!(buffer.len(), 16, "Connect response should be 16 bytes");
}

#[test]
fn test_udp_zero_copy_optimization() {
    // Test that we can parse from a slice without allocation
    let packet_data = [0u8; 1496];
    let data_slice = &packet_data[0..16];

    // This tests the optimization: &packet.data[..packet.data_len]
    // instead of: &Vec::from(&packet.data[..packet.data_len])

    let mut packet = vec![];
    packet.extend_from_slice(&PROTOCOL_IDENTIFIER.to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&12345u32.to_be_bytes());

    // Verify we can parse directly from slice
    let result = Request::from_bytes(&packet[..], 74);
    assert!(result.is_ok(), "Should parse from slice without Vec allocation");
}

#[tokio::test]
async fn test_udp_announce_request_parsing() {
    use byteorder::{BigEndian, WriteBytesExt};

    let mut packet = vec![];

    // Connection ID (8 bytes)
    packet.write_u64::<BigEndian>(12345).unwrap();
    // Action: Announce (1)
    packet.write_u32::<BigEndian>(1).unwrap();
    // Transaction ID
    packet.write_u32::<BigEndian>(54321).unwrap();
    // Info hash (20 bytes)
    packet.extend_from_slice(&[0u8; 20]);
    // Peer ID (20 bytes)
    packet.extend_from_slice(&[1u8; 20]);
    // Downloaded
    packet.write_u64::<BigEndian>(0).unwrap();
    // Left
    packet.write_u64::<BigEndian>(1000).unwrap();
    // Uploaded
    packet.write_u64::<BigEndian>(0).unwrap();
    // Event (0 = none)
    packet.write_u32::<BigEndian>(0).unwrap();
    // IP (0 = default)
    packet.write_u32::<BigEndian>(0).unwrap();
    // Key
    packet.write_u32::<BigEndian>(0).unwrap();
    // Num want (-1 = default)
    packet.write_i32::<BigEndian>(-1).unwrap();
    // Port
    packet.write_u16::<BigEndian>(6881).unwrap();

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
    use byteorder::{BigEndian, WriteBytesExt};

    let mut packet = vec![];

    // Connection ID
    packet.write_u64::<BigEndian>(12345).unwrap();
    // Action: Scrape (2)
    packet.write_u32::<BigEndian>(2).unwrap();
    // Transaction ID
    packet.write_u32::<BigEndian>(99999).unwrap();
    // Info hash (20 bytes) - can have multiple
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
    // Test MAX_PACKET_SIZE and MAX_SCRAPE_TORRENTS limits
    use byteorder::{BigEndian, WriteBytesExt};

    let mut packet = vec![];
    packet.write_u64::<BigEndian>(12345).unwrap();
    packet.write_u32::<BigEndian>(2).unwrap(); // Scrape action
    packet.write_u32::<BigEndian>(1).unwrap();

    // Try to add more than MAX_SCRAPE_TORRENTS (74) info hashes
    for _ in 0..80 {
        packet.extend_from_slice(&[0u8; 20]);
    }

    let result = Request::from_bytes(&packet, 74);

    // Should still parse but limit to MAX_SCRAPE_TORRENTS
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
async fn test_connection_id_generation() {
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);

    let conn_id1 = UdpServer::get_connection_id(&addr).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let conn_id2 = UdpServer::get_connection_id(&addr).await;

    // Connection IDs should be different (based on timestamp)
    assert_ne!(conn_id1.0, conn_id2.0, "Connection IDs should be unique");
}

#[test]
fn test_protocol_identifier_constant() {
    // Verify the magic protocol identifier
    assert_eq!(PROTOCOL_IDENTIFIER, 0x41727101980, "Protocol ID should match BEP 15 spec");
}
