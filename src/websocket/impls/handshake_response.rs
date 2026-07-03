use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::websocket::structs::handshake_request::CLUSTER_PROTOCOL_VERSION;
use crate::websocket::structs::handshake_response::HandshakeResponse;

impl HandshakeResponse {
    /// Creates the accepted-handshake reply announcing the negotiated encoding and master id.
    pub fn success(encoding: ClusterEncoding, master_id: String) -> Self {
        Self {
            success: true,
            error: None,
            encoding: Some(encoding),
            version: CLUSTER_PROTOCOL_VERSION,
            master_id: Some(master_id),
        }
    }

    /// Creates the rejected-handshake reply carrying the error message.
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