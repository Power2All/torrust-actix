//! Core BitTorrent tracker implementation.
//!
//! This module contains the main tracker logic for handling BitTorrent protocol
//! operations including peer management, torrent tracking, and announce/scrape handling.
//!
//! # Architecture
//!
//! The tracker uses a sharded architecture for scalable peer storage:
//! - Torrents are distributed across 256 shards based on the first byte of the info hash
//! - Each shard is protected by a `RwLock` for concurrent access
//! - Statistics are tracked atomically for thread-safe updates
//!
//! # Main Components
//!
//! - `TorrentTracker` - The main tracker instance
//! - `TorrentSharding` - Sharded storage for torrents
//! - `InfoHash` - 20-byte torrent identifier
//! - `PeerId` - 20-byte peer identifier
//! - `TorrentEntry` - Torrent metadata and peer lists
//! - `TorrentPeer` - Individual peer information
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::config::Configuration;
//! use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
//! use std::sync::Arc;
//!
//! // Create tracker from configuration
//! let config = Arc::new(Configuration::default());
//! let tracker = TorrentTracker::new(config, false).await;
//!
//! // Access tracker statistics
//! let stats = tracker.stats.clone();
//! ```

/// Enumerations for tracker operations.
///
/// Contains enums for announce events, peer types, and update actions.
pub mod enums;

/// Implementation blocks for tracker structs.
///
/// Contains the method implementations for all tracker-related structs
/// including the main `TorrentTracker`, sharding logic, and peer operations.
pub mod impls;

/// Data structures for tracker operations.
///
/// Contains struct definitions for torrents, peers, users, and request/response types.
pub mod structs;

/// Type aliases for complex collection types.
///
/// Defines thread-safe collection types used for tracking pending updates
/// to torrents, keys, and users.
pub mod types;

/// Unit tests for tracker functionality.
pub mod tests;