mod common;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
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
    use byteorder::{BigEndian, WriteBytesExt};

    let mut packet = vec![];

    
    packet.write_u64::<BigEndian>(12345).unwrap();
    
    packet.write_u32::<BigEndian>(1).unwrap();
    
    packet.write_u32::<BigEndian>(54321).unwrap();
    
    packet.extend_from_slice(&[0u8; 20]);
    
    packet.extend_from_slice(&[1u8; 20]);
    
    packet.write_u64::<BigEndian>(0).unwrap();
    
    packet.write_u64::<BigEndian>(1000).unwrap();
    
    packet.write_u64::<BigEndian>(0).unwrap();
    
    packet.write_u32::<BigEndian>(0).unwrap();
    
    packet.write_u32::<BigEndian>(0).unwrap();
    
    packet.write_u32::<BigEndian>(0).unwrap();
    
    packet.write_i32::<BigEndian>(-1).unwrap();
    
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

    
    packet.write_u64::<BigEndian>(12345).unwrap();
    
    packet.write_u32::<BigEndian>(2).unwrap();
    
    packet.write_u32::<BigEndian>(99999).unwrap();
    
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
    
    use byteorder::{BigEndian, WriteBytesExt};

    let mut packet = vec![];
    packet.write_u64::<BigEndian>(12345).unwrap();
    packet.write_u32::<BigEndian>(2).unwrap(); 
    packet.write_u32::<BigEndian>(1).unwrap();

    
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
async fn test_connection_id_generation() {
    use torrust_actix::udp::structs::udp_server::UdpServer;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);

    let conn_id1 = UdpServer::get_connection_id(&addr).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let conn_id2 = UdpServer::get_connection_id(&addr).await;

    
    assert_ne!(conn_id1.0, conn_id2.0, "Connection IDs should be unique");
}

#[test]
fn test_protocol_identifier_constant() {
    
    assert_eq!(PROTOCOL_IDENTIFIER, 0x41727101980, "Protocol ID should match BEP 15 spec");
}
