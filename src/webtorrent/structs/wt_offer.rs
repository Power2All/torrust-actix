use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtOffer {
    pub info_hash: String,
    pub peer_id: String,
    pub offer: String,
    pub offer_id: String,
}