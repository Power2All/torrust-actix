use crate::seeder::seeder::SharedRateLimiter;
use crate::torrent::structs::torrent_info::TorrentInfo;
use std::collections::HashMap;
use std::sync::atomic::{
    AtomicU64,
    AtomicUsize
};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

#[derive(Clone)]
pub struct TorrentRegistryEntry {
    pub torrent_info: Arc<TorrentInfo>,
    pub uploaded: Arc<AtomicU64>,
    pub peer_count: Arc<AtomicUsize>,
    pub our_peer_id: [u8; 20],
    pub rate_limiter: Option<SharedRateLimiter>,
    /// Cloned receivers of the seeder's internal stop channel.
    /// Peers clone this to get their own receiver that fires when the seeder shuts down.
    pub stop_rx: watch::Receiver<bool>,
}

pub type TorrentRegistry = Arc<RwLock<HashMap<[u8; 20], TorrentRegistryEntry>>>;

pub fn new_registry() -> TorrentRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}