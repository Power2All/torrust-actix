//! Type alias for pending API key database updates.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;

/// Thread-safe collection of pending API key database updates.
///
/// This type alias represents a concurrent map of pending key operations
/// that will be batch-written to the database. Each entry contains:
/// - Key: Unique operation ID (u128)
/// - Value: Tuple of (InfoHash as key, i64 expiration timestamp, UpdatesAction)
///
/// # Key Storage
///
/// API keys are stored using `InfoHash` (20 bytes) as the identifier,
/// with an i64 timestamp indicating when the key expires:
/// - Positive timestamp: Unix epoch expiration time
/// - Zero or negative: Key does not expire
///
/// # Thread Safety
///
/// Wrapped in `Arc<RwLock<...>>` for safe concurrent access from
/// multiple request handlers and the database sync task.
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::types::keys_updates::KeysUpdates;
///
/// // Check number of pending key updates
/// let pending = updates.read().len();
/// ```
pub type KeysUpdates = Arc<RwLock<HashMap<u128, (InfoHash, i64, UpdatesAction)>>>;