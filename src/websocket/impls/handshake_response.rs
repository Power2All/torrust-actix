use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::websocket::structs::handshake_request::CLUSTER_PROTOCOL_VERSION;
use crate::websocket::structs::handshake_response::HandshakeResponse;

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