//! Data structures for BitTorrent tracker operations.
//!
//! This module contains all the struct definitions used throughout the tracker,
//! including core identifier types, peer information, and request structures.

/// Main tracker instance struct.
///
/// The central struct that holds all tracker state including configuration,
/// database connections, torrents, users, and statistics.
pub mod torrent_tracker;

/// Announce request query parameters.
///
/// Represents the parsed query string from an announce request containing
/// info hash, peer ID, port, and transfer statistics.
pub mod announce_query_request;

/// 20-byte torrent info hash identifier.
///
/// A wrapper around `[u8; 20]` that implements common traits for use as
/// a map key and for serialization.
pub mod info_hash;

/// 20-byte peer identifier.
///
/// A wrapper around `[u8; 20]` representing the unique peer ID sent by
/// BitTorrent clients in announce requests.
pub mod peer_id;

/// Scrape request query parameters.
///
/// Represents the parsed query string from a scrape request containing
/// one or more info hashes to query.
pub mod scrape_query_request;

/// Torrent metadata and peer collections.
///
/// Contains the seeds and peers maps for a torrent, along with completion
/// count and last update timestamp.
pub mod torrent_entry;

/// Individual peer information.
///
/// Contains peer connection details including address, port, transfer stats,
/// and the last announce event.
pub mod torrent_peer;

/// User account entry data.
///
/// Stores per-user statistics including upload/download totals, completion
/// count, and active torrent tracking.
pub mod user_entry_item;

/// 20-byte user identifier.
///
/// A wrapper around `[u8; 20]` used to identify user accounts for
/// private tracker functionality.
pub mod user_id;

/// Separated IPv4/IPv6 peer collections.
///
/// Contains separate maps for IPv4 and IPv6 seeds and peers, used when
/// returning peers from announce requests.
pub mod torrent_peers;

/// Sharded torrent storage.
///
/// Implements 256-shard distribution of torrents for concurrent access
/// with minimal lock contention.
pub mod torrent_sharding;

/// Cleanup operation statistics.
///
/// Tracks the number of torrents, seeds, and peers removed during
/// periodic cleanup operations.
pub mod cleanup_stats;