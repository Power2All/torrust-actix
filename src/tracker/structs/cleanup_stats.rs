use std::sync::atomic::AtomicU64;

pub struct CleanupStats {
    pub torrents: AtomicU64,
    pub seeds: AtomicU64,
    pub peers: AtomicU64,
}