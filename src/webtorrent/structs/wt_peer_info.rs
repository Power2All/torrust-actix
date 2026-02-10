use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtPeerInfo {
    pub peer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}