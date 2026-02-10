use serde::Serialize;
use std::net::SocketAddr;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct WebTorrentPeer {
    pub peer_id: [u8; 20],
    pub peer_addr: SocketAddr,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_id: Option<String>,
    #[serde(skip)]
    pub last_announce: Instant,
    #[serde(skip)]
    pub first_announce: Instant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_seeder: Option<bool>,
}