use std::sync::atomic::AtomicU64;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TorrentEntry {
    pub seeds: AtomicU64,
    pub peers: AtomicU64,
    pub completed: AtomicU64,
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant
}
