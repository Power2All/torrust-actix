#[cfg(test)]
mod websocket_tests {
    use crate::config::enums::cluster_encoding::ClusterEncoding;
    use crate::websocket::enums::encoding_error::EncodingError;
    use crate::websocket::enums::forward_error::ForwardError;
    use crate::websocket::enums::protocol_type::ProtocolType;
    use crate::websocket::enums::request_type::RequestType;
    use crate::websocket::structs::cluster_request::ClusterRequest;
    use crate::websocket::structs::cluster_response::ClusterResponse;
    use crate::websocket::structs::handshake::{
        HandshakeRequest,
        HandshakeResponse,
        CLUSTER_PROTOCOL_VERSION
    };
    use crate::websocket::websocket::{
        create_cluster_error_response,
        create_cluster_error_response_json,
        decode,
        encode
    };
    use std::net::IpAddr;

    #[test]
    fn test_encode_decode_json() {
        let request = ClusterRequest::new(
            1,
            ProtocolType::Http,
            RequestType::Announce,
            "127.0.0.1".parse::<IpAddr>().unwrap(),
            6969,
            vec![1, 2, 3, 4],
        );
        let encoded = encode(&ClusterEncoding::json, &request).unwrap();
        let decoded: ClusterRequest = decode(&ClusterEncoding::json, &encoded).unwrap();
        assert_eq!(request.request_id, decoded.request_id);
        assert_eq!(request.protocol, decoded.protocol);
        assert_eq!(request.payload, decoded.payload);
    }

    #[test]
    fn test_encode_decode_msgpack() {
        let request = ClusterRequest::new(
            1,
            ProtocolType::Udp,
            RequestType::Scrape,
            "::1".parse::<IpAddr>().unwrap(),
            6969,
            vec![5, 6, 7, 8],
        );
        let encoded = encode(&ClusterEncoding::msgpack, &request).unwrap();
        let decoded: ClusterRequest = decode(&ClusterEncoding::msgpack, &encoded).unwrap();
        assert_eq!(request.request_id, decoded.request_id);
        assert_eq!(request.protocol, decoded.protocol);
        assert_eq!(request.payload, decoded.payload);
    }

    #[test]
    fn test_encode_decode_binary() {
        let request = ClusterRequest::new(
            1,
            ProtocolType::Api,
            RequestType::ApiCall {
                endpoint: "/api/stats".to_string(),
                method: "GET".to_string(),
            },
            "192.168.1.1".parse::<IpAddr>().unwrap(),
            8080,
            vec![],
        );
        let encoded = encode(&ClusterEncoding::binary, &request).unwrap();
        let decoded: ClusterRequest = decode(&ClusterEncoding::binary, &encoded).unwrap();
        assert_eq!(request.request_id, decoded.request_id);
        assert_eq!(request.protocol, decoded.protocol);
    }

    #[test]
    fn test_encoding_error_display_serialization() {
        let error = EncodingError::SerializationError("test error".to_string());
        assert_eq!(format!("{}", error), "Serialization error: test error");
    }

    #[test]
    fn test_encoding_error_display_deserialization() {
        let error = EncodingError::DeserializationError("invalid data".to_string());
        assert_eq!(format!("{}", error), "Deserialization error: invalid data");
    }

    #[test]
    fn test_encoding_error_debug() {
        let error = EncodingError::SerializationError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("SerializationError"));
    }

    #[test]
    fn test_decode_invalid_json() {
        let invalid_json = b"not valid json at all";
        let result: Result<ClusterRequest, _> = decode(&ClusterEncoding::json, invalid_json);
        assert!(result.is_err());
        if let Err(EncodingError::DeserializationError(msg)) = result {
            assert!(!msg.is_empty());
        } else {
            panic!("Expected DeserializationError");
        }
    }

    #[test]
    fn test_decode_invalid_msgpack() {
        let invalid_data = b"\xff\xff\xff";
        let result: Result<ClusterRequest, _> = decode(&ClusterEncoding::msgpack, invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_invalid_binary() {
        let invalid_data = b"\x00\x01\x02";
        let result: Result<ClusterRequest, _> = decode(&ClusterEncoding::binary, invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_decode_cluster_response_json() {
        let response = ClusterResponse::success(42, vec![0xde, 0xad, 0xbe, 0xef]);
        let encoded = encode(&ClusterEncoding::json, &response).unwrap();
        let decoded: ClusterResponse = decode(&ClusterEncoding::json, &encoded).unwrap();
        assert_eq!(response.request_id, decoded.request_id);
        assert_eq!(response.success, decoded.success);
        assert_eq!(response.payload, decoded.payload);
    }

    #[test]
    fn test_encode_decode_cluster_response_msgpack() {
        let response = ClusterResponse::error(99, "Something failed".to_string());
        let encoded = encode(&ClusterEncoding::msgpack, &response).unwrap();
        let decoded: ClusterResponse = decode(&ClusterEncoding::msgpack, &encoded).unwrap();
        assert_eq!(response.request_id, decoded.request_id);
        assert!(!decoded.success);
        assert_eq!(response.error_message, decoded.error_message);
    }

    #[test]
    fn test_encode_decode_handshake_request_json() {
        let request = HandshakeRequest::new("secret_token".to_string(), "slave-001".to_string());
        let encoded = encode(&ClusterEncoding::json, &request).unwrap();
        let decoded: HandshakeRequest = decode(&ClusterEncoding::json, &encoded).unwrap();
        assert_eq!(request.token, decoded.token);
        assert_eq!(request.slave_id, decoded.slave_id);
        assert_eq!(request.version, decoded.version);
    }

    #[test]
    fn test_encode_decode_handshake_response_binary() {
        let response = HandshakeResponse::success(ClusterEncoding::binary, "master-001".to_string());
        let encoded = encode(&ClusterEncoding::binary, &response).unwrap();
        let decoded: HandshakeResponse = decode(&ClusterEncoding::binary, &encoded).unwrap();
        assert!(decoded.success);
        assert_eq!(response.encoding, decoded.encoding);
        assert_eq!(response.master_id, decoded.master_id);
    }

    #[test]
    fn test_encode_decode_handshake_failure_msgpack() {
        let response = HandshakeResponse::failure("Invalid token".to_string());
        let encoded = encode(&ClusterEncoding::msgpack, &response).unwrap();
        let decoded: HandshakeResponse = decode(&ClusterEncoding::msgpack, &encoded).unwrap();
        assert!(!decoded.success);
        assert_eq!(response.error, decoded.error);
    }

    #[test]
    fn test_encode_empty_payload() {
        let request = ClusterRequest::new(
            0,
            ProtocolType::Http,
            RequestType::Announce,
            "0.0.0.0".parse::<IpAddr>().unwrap(),
            0,
            vec![],
        );
        for encoding in &[ClusterEncoding::json, ClusterEncoding::msgpack, ClusterEncoding::binary] {
            let encoded = encode(encoding, &request).unwrap();
            let decoded: ClusterRequest = decode(encoding, &encoded).unwrap();
            assert!(decoded.payload.is_empty());
        }
    }

    #[test]
    fn test_encode_large_payload() {
        let large_payload: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let request = ClusterRequest::new(
            u64::MAX,
            ProtocolType::Udp,
            RequestType::UdpPacket,
            "255.255.255.255".parse::<IpAddr>().unwrap(),
            65535,
            large_payload.clone(),
        );
        let encoded = encode(&ClusterEncoding::binary, &request).unwrap();
        let decoded: ClusterRequest = decode(&ClusterEncoding::binary, &encoded).unwrap();
        assert_eq!(decoded.payload.len(), 10000);
        assert_eq!(decoded.payload, large_payload);
    }

    #[test]
    fn test_binary_and_msgpack_produce_same_output() {
        let request = ClusterRequest::new(
            123,
            ProtocolType::Http,
            RequestType::Announce,
            "10.0.0.1".parse::<IpAddr>().unwrap(),
            6881,
            vec![1, 2, 3],
        );
        let binary_encoded = encode(&ClusterEncoding::binary, &request).unwrap();
        let msgpack_encoded = encode(&ClusterEncoding::msgpack, &request).unwrap();
        assert_eq!(binary_encoded, msgpack_encoded);
    }

    #[test]
    fn test_json_encoding_is_human_readable() {
        let response = ClusterResponse::success(42, vec![1, 2, 3]);
        let encoded = encode(&ClusterEncoding::json, &response).unwrap();
        let json_str = String::from_utf8(encoded).unwrap();
        assert!(json_str.contains("request_id"));
        assert!(json_str.contains("42"));
        assert!(json_str.contains("success"));
        assert!(json_str.contains("true"));
    }

    #[test]
    fn test_msgpack_is_more_compact_than_json() {
        let request = ClusterRequest::new(
            999999,
            ProtocolType::Api,
            RequestType::ApiCall {
                endpoint: "/api/v1/very/long/endpoint/path".to_string(),
                method: "POST".to_string(),
            },
            "192.168.100.200".parse::<IpAddr>().unwrap(),
            12345,
            vec![0; 100],
        );
        let json_encoded = encode(&ClusterEncoding::json, &request).unwrap();
        let msgpack_encoded = encode(&ClusterEncoding::msgpack, &request).unwrap();
        assert!(msgpack_encoded.len() < json_encoded.len());
    }

    #[test]
    fn test_forward_error_display_not_connected() {
        let error = ForwardError::NotConnected;
        assert_eq!(format!("{}", error), "Not connected to master");
    }

    #[test]
    fn test_forward_error_display_timeout() {
        let error = ForwardError::Timeout;
        assert_eq!(format!("{}", error), "Cluster timeout");
    }

    #[test]
    fn test_forward_error_display_master_error() {
        let error = ForwardError::MasterError("Internal server error".to_string());
        assert_eq!(format!("{}", error), "Master error: Internal server error");
    }

    #[test]
    fn test_forward_error_display_connection_lost() {
        let error = ForwardError::ConnectionLost;
        assert_eq!(format!("{}", error), "Cluster connection lost");
    }

    #[test]
    fn test_forward_error_display_encoding_error() {
        let error = ForwardError::EncodingError("Invalid msgpack".to_string());
        assert_eq!(format!("{}", error), "Encoding error: Invalid msgpack");
    }

    #[test]
    fn test_forward_error_debug() {
        let error = ForwardError::NotConnected;
        let debug_str = format!("{:?}", error);
        assert_eq!(debug_str, "NotConnected");
        let error = ForwardError::MasterError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MasterError"));
    }

    #[test]
    fn test_create_cluster_error_response_not_connected() {
        let error = ForwardError::NotConnected;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.starts_with("d14:failure reason"));
        assert!(response_str.ends_with("e"));
        assert!(response_str.contains("Cluster connection lost"));
    }

    #[test]
    fn test_create_cluster_error_response_timeout() {
        let error = ForwardError::Timeout;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Cluster timeout"));
    }

    #[test]
    fn test_create_cluster_error_response_master_error() {
        let error = ForwardError::MasterError("Custom error message".to_string());
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Custom error message"));
    }

    #[test]
    fn test_create_cluster_error_response_connection_lost() {
        let error = ForwardError::ConnectionLost;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Cluster connection lost"));
    }

    #[test]
    fn test_create_cluster_error_response_encoding_error() {
        let error = ForwardError::EncodingError("bad data".to_string());
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Cluster encoding error"));
    }

    #[test]
    fn test_create_cluster_error_response_bencode_format() {
        let error = ForwardError::Timeout;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert_eq!(response_str, "d14:failure reason15:Cluster timeoute");
    }

    #[test]
    fn test_create_cluster_error_response_json_not_connected() {
        let error = ForwardError::NotConnected;
        let response = create_cluster_error_response_json(&error);
        assert!(response.contains("Cluster connection lost"));
        assert!(response.contains("failure_reason"));
    }

    #[test]
    fn test_create_cluster_error_response_json_timeout() {
        let error = ForwardError::Timeout;
        let response = create_cluster_error_response_json(&error);
        assert!(response.contains("Cluster timeout"));
    }

    #[test]
    fn test_create_cluster_error_response_json_master_error() {
        let error = ForwardError::MasterError("Server error".to_string());
        let response = create_cluster_error_response_json(&error);
        assert!(response.contains("Server error"));
    }

    #[test]
    fn test_create_cluster_error_response_json_encoding_error() {
        let error = ForwardError::EncodingError("bad encoding".to_string());
        let response = create_cluster_error_response_json(&error);
        assert!(response.contains("Cluster encoding error"));
    }

    #[test]
    fn test_cluster_request_new() {
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        let request = ClusterRequest::new(
            42,
            ProtocolType::Http,
            RequestType::Announce,
            ip,
            6881,
            vec![1, 2, 3],
        );
        assert_eq!(request.request_id, 42);
        assert_eq!(request.protocol, ProtocolType::Http);
        assert_eq!(request.request_type, RequestType::Announce);
        assert_eq!(request.client_ip, ip);
        assert_eq!(request.client_port, 6881);
        assert_eq!(request.payload, vec![1, 2, 3]);
        assert!(request.timestamp > 0);
    }

    #[test]
    fn test_cluster_request_with_ipv6() {
        let ip: IpAddr = "::1".parse().unwrap();
        let request = ClusterRequest::new(
            1,
            ProtocolType::Udp,
            RequestType::Scrape,
            ip,
            8080,
            vec![],
        );
        assert_eq!(request.client_ip, ip);
        assert!(request.client_ip.is_ipv6());
    }

    #[test]
    fn test_cluster_request_with_api_call() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let request = ClusterRequest::new(
            100,
            ProtocolType::Api,
            RequestType::ApiCall {
                endpoint: "/api/v1/stats".to_string(),
                method: "GET".to_string(),
            },
            ip,
            443,
            vec![],
        );
        assert_eq!(request.protocol, ProtocolType::Api);
        match &request.request_type {
            RequestType::ApiCall { endpoint, method } => {
                assert_eq!(endpoint, "/api/v1/stats");
                assert_eq!(method, "GET");
            }
            _ => panic!("Expected ApiCall request type"),
        }
    }

    #[test]
    fn test_cluster_request_serialization() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let request = ClusterRequest::new(
            1,
            ProtocolType::Http,
            RequestType::Announce,
            ip,
            6969,
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ClusterRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(request.request_id, deserialized.request_id);
        assert_eq!(request.protocol, deserialized.protocol);
        assert_eq!(request.request_type, deserialized.request_type);
        assert_eq!(request.client_ip, deserialized.client_ip);
        assert_eq!(request.client_port, deserialized.client_port);
        assert_eq!(request.payload, deserialized.payload);
    }

    #[test]
    fn test_cluster_request_clone() {
        let ip: IpAddr = "192.168.0.1".parse().unwrap();
        let request = ClusterRequest::new(
            5,
            ProtocolType::Https,
            RequestType::UdpPacket,
            ip,
            12345,
            vec![1, 2, 3, 4, 5],
        );
        let cloned = request.clone();
        assert_eq!(request.request_id, cloned.request_id);
        assert_eq!(request.payload, cloned.payload);
    }

    #[test]
    fn test_cluster_request_empty_payload() {
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let request = ClusterRequest::new(
            0,
            ProtocolType::Udp,
            RequestType::Announce,
            ip,
            0,
            vec![],
        );
        assert!(request.payload.is_empty());
        assert_eq!(request.client_port, 0);
    }

    #[test]
    fn test_cluster_response_success() {
        let response = ClusterResponse::success(42, vec![1, 2, 3, 4]);
        assert_eq!(response.request_id, 42);
        assert!(response.success);
        assert_eq!(response.payload, vec![1, 2, 3, 4]);
        assert!(response.error_message.is_none());
    }

    #[test]
    fn test_cluster_response_error() {
        let response = ClusterResponse::error(99, "Something went wrong".to_string());
        assert_eq!(response.request_id, 99);
        assert!(!response.success);
        assert!(response.payload.is_empty());
        assert_eq!(response.error_message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_cluster_response_success_empty_payload() {
        let response = ClusterResponse::success(0, vec![]);
        assert!(response.success);
        assert!(response.payload.is_empty());
    }

    #[test]
    fn test_cluster_response_serialization() {
        let response = ClusterResponse::success(123, vec![0xca, 0xfe]);
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: ClusterResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response.request_id, deserialized.request_id);
        assert_eq!(response.success, deserialized.success);
        assert_eq!(response.payload, deserialized.payload);
        assert_eq!(response.error_message, deserialized.error_message);
    }

    #[test]
    fn test_cluster_response_error_serialization() {
        let response = ClusterResponse::error(456, "Error message".to_string());
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: ClusterResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response.request_id, deserialized.request_id);
        assert!(!deserialized.success);
        assert_eq!(response.error_message, deserialized.error_message);
    }

    #[test]
    fn test_cluster_response_clone() {
        let response = ClusterResponse::success(1, vec![10, 20, 30]);
        let cloned = response.clone();
        assert_eq!(response.request_id, cloned.request_id);
        assert_eq!(response.payload, cloned.payload);
    }

    #[test]
    fn test_cluster_response_debug() {
        let response = ClusterResponse::success(1, vec![]);
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("ClusterResponse"));
        assert!(debug_str.contains("request_id"));
        assert!(debug_str.contains("success: true"));
    }

    #[test]
    fn test_handshake_request_new() {
        let request = HandshakeRequest::new("my_token".to_string(), "slave_001".to_string());
        assert_eq!(request.token, "my_token");
        assert_eq!(request.slave_id, "slave_001");
        assert_eq!(request.version, crate::websocket::structs::handshake::CLUSTER_PROTOCOL_VERSION);
    }

    #[test]
    fn test_handshake_request_serialization() {
        let request = HandshakeRequest::new("secret".to_string(), "slave_123".to_string());
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: HandshakeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(request.token, deserialized.token);
        assert_eq!(request.slave_id, deserialized.slave_id);
        assert_eq!(request.version, deserialized.version);
    }

    #[test]
    fn test_handshake_response_success() {
        use crate::config::enums::cluster_encoding::ClusterEncoding;
        let response = HandshakeResponse::success(ClusterEncoding::json, "master_001".to_string());
        assert!(response.success);
        assert_eq!(response.encoding, Some(ClusterEncoding::json));
        assert_eq!(response.master_id, Some("master_001".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_handshake_response_failure() {
        let response = HandshakeResponse::failure("Authentication failed".to_string());
        assert!(!response.success);
        assert_eq!(response.error, Some("Authentication failed".to_string()));
        assert!(response.encoding.is_none());
        assert!(response.master_id.is_none());
    }

    #[test]
    fn test_handshake_response_success_serialization() {
        use crate::config::enums::cluster_encoding::ClusterEncoding;
        let response = HandshakeResponse::success(ClusterEncoding::msgpack, "master_abc".to_string());
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: HandshakeResponse = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.encoding, Some(ClusterEncoding::msgpack));
        assert_eq!(deserialized.master_id, Some("master_abc".to_string()));
    }

    #[test]
    fn test_handshake_response_failure_serialization() {
        let response = HandshakeResponse::failure("Invalid token".to_string());
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: HandshakeResponse = serde_json::from_str(&serialized).unwrap();
        assert!(!deserialized.success);
        assert_eq!(deserialized.error, Some("Invalid token".to_string()));
    }

    #[test]
    fn test_handshake_response_default_version() {
        let response = HandshakeResponse {
            success: true,
            version: 0,
            encoding: None,
            master_id: None,
            error: None,
        };
        assert_eq!(response.version, 0);
        assert!(response.success);
    }

    #[test]
    fn test_cluster_protocol_version() {
        assert_eq!(CLUSTER_PROTOCOL_VERSION, 1);
    }

    #[test]
    fn test_handshake_response_success_json_encoding() {
        let response = HandshakeResponse::success(ClusterEncoding::json, "master-json".to_string());
        assert_eq!(response.encoding, Some(ClusterEncoding::json));
    }

    #[test]
    fn test_handshake_response_success_msgpack_encoding() {
        let response = HandshakeResponse::success(ClusterEncoding::msgpack, "master-msgpack".to_string());
        assert_eq!(response.encoding, Some(ClusterEncoding::msgpack));
    }

    #[test]
    fn test_handshake_request_clone() {
        let request = HandshakeRequest::new("token".to_string(), "slave".to_string());
        let cloned = request.clone();
        assert_eq!(request.token, cloned.token);
        assert_eq!(request.slave_id, cloned.slave_id);
    }

    #[test]
    fn test_handshake_response_clone() {
        let response = HandshakeResponse::success(ClusterEncoding::json, "master".to_string());
        let cloned = response.clone();
        assert_eq!(response.success, cloned.success);
        assert_eq!(response.encoding, cloned.encoding);
    }

    #[test]
    fn test_handshake_request_empty_values() {
        let request = HandshakeRequest::new(String::new(), String::new());
        assert!(request.token.is_empty());
        assert!(request.slave_id.is_empty());
        assert_eq!(request.version, CLUSTER_PROTOCOL_VERSION);
    }

    #[test]
    fn test_protocol_type_serialization() {
        let http = ProtocolType::Http;
        let serialized = serde_json::to_string(&http).unwrap();
        assert_eq!(serialized, "\"Http\"");
        let https = ProtocolType::Https;
        let serialized = serde_json::to_string(&https).unwrap();
        assert_eq!(serialized, "\"Https\"");
        let udp = ProtocolType::Udp;
        let serialized = serde_json::to_string(&udp).unwrap();
        assert_eq!(serialized, "\"Udp\"");
        let api = ProtocolType::Api;
        let serialized = serde_json::to_string(&api).unwrap();
        assert_eq!(serialized, "\"Api\"");
    }

    #[test]
    fn test_protocol_type_deserialization() {
        let http: ProtocolType = serde_json::from_str("\"Http\"").unwrap();
        assert_eq!(http, ProtocolType::Http);
        let https: ProtocolType = serde_json::from_str("\"Https\"").unwrap();
        assert_eq!(https, ProtocolType::Https);
        let udp: ProtocolType = serde_json::from_str("\"Udp\"").unwrap();
        assert_eq!(udp, ProtocolType::Udp);
        let api: ProtocolType = serde_json::from_str("\"Api\"").unwrap();
        assert_eq!(api, ProtocolType::Api);
    }

    #[test]
    fn test_protocol_type_equality() {
        assert_eq!(ProtocolType::Http, ProtocolType::Http);
        assert_ne!(ProtocolType::Http, ProtocolType::Https);
        assert_ne!(ProtocolType::Udp, ProtocolType::Api);
    }

    #[test]
    fn test_protocol_type_clone() {
        let http = ProtocolType::Http;
        let cloned = http.clone();
        assert_eq!(http, cloned);
    }

    #[test]
    fn test_protocol_type_debug() {
        let http = ProtocolType::Http;
        let debug_str = format!("{:?}", http);
        assert_eq!(debug_str, "Http");
    }

    #[test]
    fn test_request_type_announce() {
        let announce = RequestType::Announce;
        let serialized = serde_json::to_string(&announce).unwrap();
        assert_eq!(serialized, "\"Announce\"");
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RequestType::Announce);
    }

    #[test]
    fn test_request_type_scrape() {
        let scrape = RequestType::Scrape;
        let serialized = serde_json::to_string(&scrape).unwrap();
        assert_eq!(serialized, "\"Scrape\"");
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RequestType::Scrape);
    }

    #[test]
    fn test_request_type_api_call() {
        let api_call = RequestType::ApiCall {
            endpoint: "/api/v1/stats".to_string(),
            method: "GET".to_string(),
        };
        let serialized = serde_json::to_string(&api_call).unwrap();
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            RequestType::ApiCall { endpoint, method } => {
                assert_eq!(endpoint, "/api/v1/stats");
                assert_eq!(method, "GET");
            }
            _ => panic!("Expected ApiCall variant"),
        }
    }

    #[test]
    fn test_request_type_udp_packet() {
        let udp = RequestType::UdpPacket;
        let serialized = serde_json::to_string(&udp).unwrap();
        assert_eq!(serialized, "\"UdpPacket\"");
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RequestType::UdpPacket);
    }

    #[test]
    fn test_request_type_equality() {
        assert_eq!(RequestType::Announce, RequestType::Announce);
        assert_ne!(RequestType::Announce, RequestType::Scrape);
        let api1 = RequestType::ApiCall {
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
        };
        let api2 = RequestType::ApiCall {
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
        };
        let api3 = RequestType::ApiCall {
            endpoint: "/other".to_string(),
            method: "POST".to_string(),
        };
        assert_eq!(api1, api2);
        assert_ne!(api1, api3);
    }

    #[test]
    fn test_request_type_clone() {
        let api_call = RequestType::ApiCall {
            endpoint: "/clone/test".to_string(),
            method: "DELETE".to_string(),
        };
        let cloned = api_call.clone();
        assert_eq!(api_call, cloned);
    }

    #[test]
    fn test_request_type_debug() {
        let announce = RequestType::Announce;
        let debug_str = format!("{:?}", announce);
        assert_eq!(debug_str, "Announce");
        let api_call = RequestType::ApiCall {
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
        };
        let debug_str = format!("{:?}", api_call);
        assert!(debug_str.contains("ApiCall"));
        assert!(debug_str.contains("/test"));
        assert!(debug_str.contains("GET"));
    }
}