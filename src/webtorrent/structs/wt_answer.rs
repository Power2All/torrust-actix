use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtAnswer {
    pub info_hash: String,
    pub peer_id: String,
    pub answer: String,
    pub offer_id: String,
    pub to_peer_id: String,
}