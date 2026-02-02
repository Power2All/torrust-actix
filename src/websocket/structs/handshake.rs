use crate::config::enums::cluster_encoding::ClusterEncoding;
use serde::{Deserialize, Serialize};

pub const CLUSTER_PROTOCOL_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeRequest {
    pub token: String,
    pub slave_id: String,
    pub version: u8,
}

impl HandshakeRequest {
    pub fn new(token: String, slave_id: String) -> Self {
        Self {
            token,
            slave_id,
            version: CLUSTER_PROTOCOL_VERSION,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeResponse {
    pub success: bool,
    pub error: Option<String>,
    pub encoding: Option<ClusterEncoding>,
    pub version: u8,
    pub master_id: Option<String>,
}

impl HandshakeResponse {
    pub fn success(encoding: ClusterEncoding, master_id: String) -> Self {
        Self {
            success: true,
            error: None,
            encoding: Some(encoding),
            version: CLUSTER_PROTOCOL_VERSION,
            master_id: Some(master_id),
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            encoding: None,
            version: CLUSTER_PROTOCOL_VERSION,
            master_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_protocol_version() {
        assert_eq!(CLUSTER_PROTOCOL_VERSION, 1);
    }

    #[test]
    fn test_handshake_request_new() {
        let request = HandshakeRequest::new(
            "secret_token".to_string(),
            "slave-001".to_string(),
        );
        assert_eq!(request.token, "secret_token");
        assert_eq!(request.slave_id, "slave-001");
        assert_eq!(request.version, CLUSTER_PROTOCOL_VERSION);
    }

    #[test]
    fn test_handshake_request_serialization() {
        let request = HandshakeRequest::new(
            "my_token".to_string(),
            "slave-abc".to_string(),
        );
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: HandshakeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(request.token, deserialized.token);
        assert_eq!(request.slave_id, deserialized.slave_id);
        assert_eq!(request.version, deserialized.version);
    }

    #[test]
    fn test_handshake_response_success() {
        let response = HandshakeResponse::success(
            ClusterEncoding::binary,
            "master-001".to_string(),
        );
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.encoding, Some(ClusterEncoding::binary));
        assert_eq!(response.version, CLUSTER_PROTOCOL_VERSION);
        assert_eq!(response.master_id, Some("master-001".to_string()));
    }

    #[test]
    fn test_handshake_response_success_json_encoding() {
        let response = HandshakeResponse::success(
            ClusterEncoding::json,
            "master-json".to_string(),
        );
        assert_eq!(response.encoding, Some(ClusterEncoding::json));
    }

    #[test]
    fn test_handshake_response_success_msgpack_encoding() {
        let response = HandshakeResponse::success(
            ClusterEncoding::msgpack,
            "master-msgpack".to_string(),
        );
        assert_eq!(response.encoding, Some(ClusterEncoding::msgpack));
    }

    #[test]
    fn test_handshake_response_failure() {
        let response = HandshakeResponse::failure("Invalid token".to_string());
        assert!(!response.success);
        assert_eq!(response.error, Some("Invalid token".to_string()));
        assert!(response.encoding.is_none());
        assert_eq!(response.version, CLUSTER_PROTOCOL_VERSION);
        assert!(response.master_id.is_none());
    }

    #[test]
    fn test_handshake_response_serialization() {
        let response = HandshakeResponse::success(
            ClusterEncoding::binary,
            "master-test".to_string(),
        );
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: HandshakeResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response.success, deserialized.success);
        assert_eq!(response.encoding, deserialized.encoding);
        assert_eq!(response.master_id, deserialized.master_id);
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
}