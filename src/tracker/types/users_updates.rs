use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

/// Pending user writes, one per user: a newer update replaces the queued one, so the queue holds
/// at most one entry per active user instead of one per announce. The `u128` is the update's
/// sequence number, which tells a flush whether the entry was replaced while it was being saved.
pub type UsersUpdates = Arc<RwLock<HashMap<UserId, (u128, UserEntryItem, UpdatesAction)>>>;