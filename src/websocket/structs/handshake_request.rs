use serde::{
    Deserialize,
    Serialize
};

pub const CLUSTER_PROTOCOL_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeRequest {
    pub token: String,
    pub slave_id: String,
    pub version: u8,
}