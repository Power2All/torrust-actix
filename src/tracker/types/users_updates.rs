use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

pub type UsersUpdates = Arc<RwLock<HashMap<u128, (UserId, UserEntryItem, UpdatesAction)>>>;