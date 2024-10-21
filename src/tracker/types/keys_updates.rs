use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;

pub type KeysUpdates = Arc<RwLock<HashMap<u128, (InfoHash, i64, UpdatesAction)>>>;