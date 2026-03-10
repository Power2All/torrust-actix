use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::user_id::UserId;
use serde::{
    Deserialize,
    Serialize
};
use std::collections::BTreeMap;

/// Per-user statistics and state stored in the tracker.
///
/// Users are looked up by their [`UserId`] (derived from the per-user announce
/// key) and can optionally be tied to a database row via `user_id` or
/// `user_uuid`.
///
/// [`UserId`]: crate::tracker::structs::user_id::UserId
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    /// The user's unique 20-byte key (used as the in-memory map key).
    pub key: UserId,
    /// Optional numeric database primary key.
    pub user_id: Option<u64>,
    /// Optional UUID database identifier.
    pub user_uuid: Option<String>,
    /// Cumulative bytes uploaded across all torrents.
    pub uploaded: u64,
    /// Cumulative bytes downloaded across all torrents.
    pub downloaded: u64,
    /// Number of torrents fully downloaded by this user.
    pub completed: u64,
    /// Unix timestamp of the last announce from this user.
    pub updated: u64,
    /// Whether this user account is enabled (`1`) or disabled (`0`).
    pub active: u8,
    /// Torrents the user is currently seeding or leeching, mapped to their
    /// last-announce timestamp.
    pub torrents_active: BTreeMap<InfoHash, u64>
}