use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
pub struct GetTorrentApi {
    pub info_hash: String,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
    pub peers: Vec<Value>,
}