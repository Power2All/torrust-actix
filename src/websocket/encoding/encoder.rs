use serde::{de::DeserializeOwned, Serialize};
use crate::config::enums::cluster_encoding::ClusterEncoding;

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

/// Encode a message using the specified encoding format
pub fn encode<T: Serialize>(encoding: &ClusterEncoding, value: &T) -> Result<Vec<u8>, EncodingError> {
    match encoding {
        ClusterEncoding::binary => encode_binary(value),
        ClusterEncoding::json => encode_json(value),
        ClusterEncoding::msgpack => encode_msgpack(value),
    }
}

/// Decode a message using the specified encoding format
pub fn decode<T: DeserializeOwned>(encoding: &ClusterEncoding, data: &[u8]) -> Result<T, EncodingError> {
    match encoding {
        ClusterEncoding::binary => decode_binary(data),
        ClusterEncoding::json => decode_json(data),
        ClusterEncoding::msgpack => decode_msgpack(data),
    }
}

/// Binary encoding using bincode-style format (simple length-prefixed binary)
fn encode_binary<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    // Use MessagePack as the binary format since it's compact and well-supported
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
    use crate::websocket::structs::cluster_request::ClusterRequest;
    use crate::websocket::enums::protocol_type::ProtocolType;
    use crate::websocket::enums::request_type::RequestType;
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
}
