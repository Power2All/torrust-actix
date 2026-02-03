//! Sharded torrent storage for concurrent access.

use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Sharded storage for torrent entries with 256 shards.
///
/// `TorrentSharding` distributes torrents across 256 separate shards to minimize
/// lock contention in high-concurrency scenarios. Each shard is independently
/// locked, allowing multiple threads to access different torrents simultaneously.
///
/// # Sharding Strategy
///
/// Torrents are assigned to shards based on the first byte of their info hash:
/// - `shard_index = info_hash[0]` (0-255)
///
/// This provides uniform distribution since info hashes are SHA-1 hashes with
/// pseudo-random byte distribution.
///
/// # Thread Safety
///
/// Each shard is wrapped in `Arc<RwLock<...>>` using `parking_lot`:
/// - Multiple readers can access the same shard concurrently
/// - Writers get exclusive access to their shard only
/// - Other shards remain accessible during writes
///
/// # Performance
///
/// With 256 shards, the probability of lock contention is reduced by ~256x
/// compared to a single lock. For a tracker handling millions of torrents,
/// this significantly improves throughput.
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::structs::torrent_sharding::TorrentSharding;
/// use torrust_actix::tracker::structs::info_hash::InfoHash;
///
/// let sharding = TorrentSharding::new();
///
/// // Check if a torrent exists
/// let info_hash = InfoHash([0u8; 20]);
/// let exists = sharding.contains_torrent(info_hash);
///
/// // Get total torrent count across all shards
/// let total = sharding.get_torrents_amount();
/// ```
#[derive(Debug)]
pub struct TorrentSharding {
    /// Array of 256 shards, each containing a map of info hash to torrent entry.
    ///
    /// Shard assignment: `shards[info_hash.0[0]]`
    pub shards: [Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>; 256],
}