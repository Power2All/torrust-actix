use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A fixed array of 256 independently-locked torrent maps.
///
/// Each [`InfoHash`] is routed to a shard by its first byte, so concurrent
/// announce requests for different torrents rarely contend on the same lock.
/// The `parking_lot` [`RwLock`] is used for low-latency read-heavy workloads.
///
/// [`InfoHash`]: crate::tracker::structs::info_hash::InfoHash
/// [`RwLock`]: parking_lot::RwLock
#[derive(Debug)]
pub struct TorrentSharding {
    /// The 256 shard maps, indexed by `info_hash[0]`.
    pub shards: [Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>; 256],
}