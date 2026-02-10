use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtAnswerResponse {
    pub info_hash: String,
    pub peer_id: String,
    pub to_peer_id: String,
    pub offer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}