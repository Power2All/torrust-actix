use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GetTorrentsApi {
    pub info_hash: String,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}