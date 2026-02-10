use crate::webtorrent::structs::wt_peer_info::WtPeerInfo;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtAnnounceResponse {
    pub info_hash: String,
    pub complete: i64,
    pub incomplete: i64,
    pub peers: Vec<WtPeerInfo>,
    pub interval: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning_message: Option<String>,
}