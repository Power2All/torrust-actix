//! Statistics for periodic peer cleanup operations.

use std::sync::atomic::AtomicU64;

/// Atomic counters for tracking cleanup operation results.
///
/// `CleanupStats` is used during periodic peer timeout cleanup to track
/// the number of expired entries removed. Multiple cleanup threads can
/// safely increment these counters concurrently.
///
/// # Usage
///
/// During cleanup:
/// 1. Each thread processes assigned shards
/// 2. Counters are incremented atomically as entries are removed
/// 3. After cleanup completes, stats are applied to the main tracker
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::structs::cleanup_stats::CleanupStats;
/// use std::sync::atomic::Ordering;
///
/// let stats = CleanupStats::new();
///
/// // Increment during cleanup
/// stats.add_torrents(5);
/// stats.add_seeds(100);
/// stats.add_peers(50);
///
/// // Read final values
/// let removed_torrents = stats.torrents.load(Ordering::Relaxed);
/// ```
pub struct CleanupStats {
    /// Number of torrent entries removed during cleanup.
    pub torrents: AtomicU64,

    /// Number of seeding peers removed due to timeout.
    pub seeds: AtomicU64,

    /// Number of downloading peers removed due to timeout.
    pub peers: AtomicU64,
}