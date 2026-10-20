use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;

/// Pending announce-key writes, one per key; see [`UsersUpdates`](super::users_updates::UsersUpdates)
/// for the role of the `u128` sequence number.
pub type KeysUpdates = Arc<RwLock<HashMap<InfoHash, (u128, i64, UpdatesAction)>>>;