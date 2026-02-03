//! Type alias for pending torrent database updates.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

/// Thread-safe collection of pending torrent database updates.
///
/// This type alias represents a concurrent map of pending torrent operations
/// that will be batch-written to the database. Each entry contains:
/// - Key: Unique operation ID (u128)
/// - Value: Tuple of (InfoHash, TorrentEntry, UpdatesAction)
///
/// # Thread Safety
///
/// Wrapped in `Arc<RwLock<...>>` for safe concurrent access:
/// - Multiple readers can check pending updates
/// - Writers get exclusive access when adding/removing updates
///
/// # Operation Flow
///
/// 1. Announce requests add/update torrents in memory
/// 2. Changes are queued in this collection
/// 3. A background task periodically flushes updates to the database
/// 4. After successful write, entries are removed
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::types::torrents_updates::TorrentsUpdates;
///
/// // Check number of pending updates
/// let pending = updates.read().len();
/// ```
pub type TorrentsUpdates = Arc<RwLock<HashMap<u128, (InfoHash, TorrentEntry, UpdatesAction)>>>;