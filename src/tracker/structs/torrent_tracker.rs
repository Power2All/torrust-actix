use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use crate::tracker::types::keys_updates::KeysUpdates;
use crate::tracker::types::torrents_updates::TorrentsUpdates;
use crate::tracker::types::users_updates::UsersUpdates;

#[derive(Debug)]
pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub sqlx: DatabaseConnector,
    pub torrents_sharding: Arc<TorrentSharding>,
    pub torrents_updates: TorrentsUpdates,
    
    pub torrents_whitelist: Arc<RwLock<HashSet<InfoHash>>>,
    pub torrents_whitelist_updates: Arc<RwLock<HashMap<u128, (InfoHash, UpdatesAction)>>>,
    
    pub torrents_blacklist: Arc<RwLock<HashSet<InfoHash>>>,
    pub torrents_blacklist_updates: Arc<RwLock<HashMap<u128, (InfoHash, UpdatesAction)>>>,
    pub keys: Arc<RwLock<BTreeMap<InfoHash, i64>>>,
    pub keys_updates: KeysUpdates,
    pub users: Arc<RwLock<BTreeMap<UserId, UserEntryItem>>>,
    pub users_updates: UsersUpdates,
    pub stats: Arc<StatsAtomics>
}