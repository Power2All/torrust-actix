use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct TorrentStats {
    pub uploaded: u64,
    pub peer_count: usize,
}

pub type SharedStats = Arc<RwLock<HashMap<String, TorrentStats>>>;

pub fn new_shared_stats() -> SharedStats {
    Arc::new(RwLock::new(HashMap::new()))
}