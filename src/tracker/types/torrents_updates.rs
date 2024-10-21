use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

pub type TorrentsUpdates = Arc<RwLock<HashMap<u128, (InfoHash, TorrentEntry, UpdatesAction)>>>;