//! Type aliases for thread-safe update collections.
//!
//! These types represent pending database updates that are batched and
//! periodically flushed to the database for efficiency.

/// Pending torrent updates collection.
///
/// Thread-safe map of pending torrent insertions, updates, and deletions
/// keyed by a unique operation ID.
pub mod torrents_updates;

/// Pending API key updates collection.
///
/// Thread-safe map of pending key insertions, updates, and deletions
/// keyed by a unique operation ID.
pub mod keys_updates;

/// Pending user updates collection.
///
/// Thread-safe map of pending user insertions, updates, and deletions
/// keyed by a unique operation ID.
pub mod users_updates;