use crate::seeder::seeder::SharedRateLimiter;
use crate::torrent::structs::torrent_info::TorrentInfo;
use std::collections::HashMap;
use std::sync::atomic::{
    AtomicU64,
    AtomicUsize
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TorrentRegistryEntry {
    pub torrent_info: Arc<TorrentInfo>,
    pub uploaded: Arc<AtomicU64>,
    pub peer_count: Arc<AtomicUsize>,
    pub our_peer_id: [u8; 20],
    pub rate_limiter: Option<SharedRateLimiter>,
}

pub type TorrentRegistry = Arc<RwLock<HashMap<[u8; 20], TorrentRegistryEntry>>>;

pub fn new_registry() -> TorrentRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}