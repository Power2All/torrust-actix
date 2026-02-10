use crate::webtorrent::webtorrent::{
    default_u16,
    default_u64
};
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtAnnounce {
    pub info_hash: String,
    pub peer_id: String,
    #[serde(default = "default_u16")]
    pub port: u16,
    #[serde(default = "default_u64")]
    pub uploaded: u64,
    #[serde(default = "default_u64")]
    pub downloaded: u64,
    #[serde(default)]
    pub left: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numwant: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offers_only: Option<bool>,
}