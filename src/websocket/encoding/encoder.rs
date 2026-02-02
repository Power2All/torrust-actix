use crate::config::enums::cluster_encoding::ClusterEncoding;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug)]
pub enum EncodingError {
    SerializationError(String),
    DeserializationError(String),
}

impl std::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            EncodingError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for EncodingError {}

pub fn encode<T: Serialize>(encoding: &ClusterEncoding, value: &T) -> Result<Vec<u8>, EncodingError> {
    match encoding {
        ClusterEncoding::binary => encode_binary(value),
        ClusterEncoding::json => encode_json(value),
        ClusterEncoding::msgpack => encode_msgpack(value),
    }
}

pub fn decode<T: DeserializeOwned>(encoding: &ClusterEncoding, data: &[u8]) -> Result<T, EncodingError> {
    match encoding {
        ClusterEncoding::binary => decode_binary(data),
        ClusterEncoding::json => decode_json(data),
        ClusterEncoding::msgpack => decode_msgpack(data),
    }
}

fn encode_binary<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    rmp_serde::to_vec(value)
        .map_err(|e| EncodingError::SerializationError(e.to_string()))
}

fn decode_binary<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    rmp_serde::from_slice(data)
        .map_err(|e| EncodingError::DeserializationError(e.to_string()))
}

fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    serde_json::to_vec(value)
        .map_err(|e| EncodingError::SerializationError(e.to_string()))
}

fn decode_json<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    serde_json::from_slice(data)
        .map_err(|e| EncodingError::DeserializationError(e.to_string()))
}

fn encode_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    rmp_serde::to_vec(value)
        .map_err(|e| EncodingError::SerializationError(e.to_string()))
}

fn decode_msgpack<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    rmp_serde::from_slice(data)
        .map_err(|e| EncodingError::DeserializationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::enums::protocol_type::ProtocolType;
    use crate::websocket::enums::request_type::RequestType;
    use crate::websocket::structs::cluster_request::ClusterRequest;
    use crate::websocket::structs::cluster_response::ClusterResponse;
    use crate::websocket::structs::handshake::{HandshakeRequest, HandshakeResponse};
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
}