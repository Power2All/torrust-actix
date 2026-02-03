//! Implementation blocks for tracker data structures.
//!
//! This module contains all the `impl` blocks that provide methods for the
//! tracker's data structures. Implementations are organized by the struct
//! they extend.

/// InfoHash implementation: Display, FromStr, Serialize, Deserialize.
pub mod info_hash;

/// PeerId implementation: Display, FromStr, Serialize, client detection.
pub mod peer_id;

/// TorrentEntry implementation: creation and modification methods.
pub mod torrent_entry;

/// TorrentPeer implementation: creation and update methods.
pub mod torrent_peer;

/// TorrentTracker core implementation: initialization and configuration.
pub mod torrent_tracker;

/// TorrentTracker API key management methods.
pub mod torrent_tracker_keys;

/// TorrentTracker peer management methods (add, update, remove peers).
pub mod torrent_tracker_peers;

/// TorrentTracker torrent management methods (add, get, remove torrents).
pub mod torrent_tracker_torrents;

/// TorrentTracker announce/scrape request handlers.
pub mod torrent_tracker_handlers;

/// TorrentTracker blacklist management methods.
pub mod torrent_tracker_torrents_blacklist;

/// TorrentTracker torrent database update flushing.
pub mod torrent_tracker_torrents_updates;

/// TorrentTracker whitelist management methods.
pub mod torrent_tracker_torrents_whitelist;

/// TorrentTracker user management methods.
pub mod torrent_tracker_users;

/// TorrentTracker user database update flushing.
pub mod torrent_tracker_users_updates;

/// UserId implementation: Display, FromStr, Serialize, Deserialize.
pub mod user_id;

/// AnnounceEvent implementation: conversion methods.
pub mod announce_event;

/// TorrentSharding implementation: sharded storage operations.
pub mod torrent_sharding;

/// TorrentTracker data import from JSON files.
pub mod torrent_tracker_import;

/// TorrentTracker data export to JSON files.
pub mod torrent_tracker_export;

/// TorrentTracker self-signed certificate generation.
pub mod torrent_tracker_cert_gen;

/// TorrentTracker blacklist database update flushing.
pub mod torrent_tracker_torrents_blacklist_updates;

/// TorrentTracker whitelist database update flushing.
pub mod torrent_tracker_torrents_whitelist_updates;

/// TorrentTracker API key database update flushing.
pub mod torrent_tracker_keys_updates;

/// CleanupStats implementation: atomic counter operations.
pub mod cleanup_stats;