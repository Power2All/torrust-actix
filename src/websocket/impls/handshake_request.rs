use crate::websocket::structs::handshake_request::{
    HandshakeRequest,
    CLUSTER_PROTOCOL_VERSION,
};

impl HandshakeRequest {
    pub fn new(token: String, slave_id: String) -> Self {
        Self {
            token,
            slave_id,
            version: CLUSTER_PROTOCOL_VERSION,
        }
    }
}