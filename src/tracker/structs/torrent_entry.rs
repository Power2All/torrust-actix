//! Torrent entry with peer collections and metadata.

use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use ahash::AHasher;
use serde::Serialize;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// A high-performance HashMap using the aHash algorithm.
///
/// This type alias provides faster hashing compared to the standard HashMap
/// by using the aHash algorithm which is optimized for HashDoS resistance
/// and performance.
pub type AHashMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// A torrent entry containing peer lists and metadata.
///
/// Each `TorrentEntry` represents a tracked torrent and contains:
/// - Separate maps for seeds (complete) and peers (incomplete)
/// - Completion counter for the number of times the torrent was completed
/// - Last update timestamp for cleanup purposes
///
/// # Peer Storage
///
/// Peers are stored in high-performance hash maps keyed by their peer ID.
/// Seeds (clients with 100% of the torrent) and peers (clients still downloading)
/// are stored separately for efficient announce responses.
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::structs::torrent_entry::TorrentEntry;
///
/// // Get peer counts
/// let seeds_count = entry.seeds.len();
/// let peers_count = entry.peers.len();
/// let completions = entry.completed;
/// ```
#[derive(Serialize, Clone, Debug)]
pub struct TorrentEntry {
    /// Map of seeding peers (those with left=0).
    ///
    /// Skipped during serialization to reduce payload size.
    #[serde(skip_serializing)]
    pub seeds: AHashMap<PeerId, TorrentPeer>,

    /// Map of downloading peers (those with left>0).
    ///
    /// Skipped during serialization to reduce payload size.
    #[serde(skip_serializing)]
    pub peers: AHashMap<PeerId, TorrentPeer>,

    /// Number of times this torrent has been completed (snatched).
    pub completed: u64,

    /// Timestamp of the last peer activity (used for cleanup).
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant,
}