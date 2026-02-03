//! Type alias for pending user database updates.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

/// Thread-safe collection of pending user database updates.
///
/// This type alias represents a concurrent map of pending user operations
/// that will be batch-written to the database. Each entry contains:
/// - Key: Unique operation ID (u128)
/// - Value: Tuple of (UserId, UserEntryItem, UpdatesAction)
///
/// # User Statistics
///
/// User updates typically include changes to:
/// - Upload/download byte counts
/// - Completion counts
/// - Active torrent lists
/// - Account status changes
///
/// # Private Tracker Mode
///
/// This collection is primarily used when the tracker operates in private
/// mode with user authentication. Updates are batched to reduce database
/// load from frequent announce requests.
///
/// # Thread Safety
///
/// Wrapped in `Arc<RwLock<...>>` for safe concurrent access from
/// multiple announce handlers and the database sync task.
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::types::users_updates::UsersUpdates;
///
/// // Check number of pending user updates
/// let pending = updates.read().len();
/// ```
pub type UsersUpdates = Arc<RwLock<HashMap<u128, (UserId, UserEntryItem, UpdatesAction)>>>;