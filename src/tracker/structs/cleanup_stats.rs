use std::sync::atomic::AtomicU64;

/// Counters populated during a single peer-cleanup pass.
///
/// After each cleanup run the counts are logged to the console and then
/// reset to zero in preparation for the next interval.
pub struct CleanupStats {
    /// Number of torrents whose peer maps were inspected.
    pub torrents: AtomicU64,
    /// Number of timed-out seed entries removed.
    pub seeds: AtomicU64,
    /// Number of timed-out peer entries removed.
    pub peers: AtomicU64,
}