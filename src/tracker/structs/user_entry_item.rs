//! User account entry for private tracker functionality.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::user_id::UserId;

/// User account information for private tracker mode.
///
/// `UserEntryItem` stores per-user statistics and settings for private tracker
/// functionality. Each user is identified by a unique passkey and can have
/// their upload/download tracked for ratio enforcement.
///
/// # Statistics Tracking
///
/// The tracker maintains cumulative statistics for each user:
/// - `uploaded`: Total bytes uploaded across all torrents
/// - `downloaded`: Total bytes downloaded across all torrents
/// - `completed`: Number of torrents fully downloaded (snatched)
///
/// # Active Torrents
///
/// The `torrents_active` map tracks which torrents the user is currently
/// participating in, along with timestamps for activity tracking.
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::structs::user_entry_item::UserEntryItem;
///
/// // Calculate user ratio
/// let ratio = if user.downloaded > 0 {
///     user.uploaded as f64 / user.downloaded as f64
/// } else {
///     f64::INFINITY
/// };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    /// The user's passkey identifier (20 bytes).
    pub key: UserId,

    /// Optional numeric user ID from external system.
    pub user_id: Option<u64>,

    /// Optional UUID string for external system integration.
    pub user_uuid: Option<String>,

    /// Total bytes uploaded by this user (all torrents combined).
    pub uploaded: u64,

    /// Total bytes downloaded by this user (all torrents combined).
    pub downloaded: u64,

    /// Number of torrents completed (snatched) by this user.
    pub completed: u64,

    /// Unix timestamp of last activity.
    pub updated: u64,

    /// Account active status (0 = disabled, 1 = active).
    pub active: u8,

    /// Map of active torrents to their last activity timestamps.
    pub torrents_active: BTreeMap<InfoHash, u64>,
}