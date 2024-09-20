use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub sqlx: DatabaseConnector,
    pub torrents_sharding: Arc<TorrentSharding>,
    pub torrents_updates: Arc<RwLock<HashMap<u128, (InfoHash, TorrentEntry)>>>,
    pub torrents_whitelist: Arc<RwLock<Vec<InfoHash>>>,
    pub torrents_blacklist: Arc<RwLock<Vec<InfoHash>>>,
    pub keys: Arc<RwLock<BTreeMap<InfoHash, i64>>>,
    pub users: Arc<RwLock<BTreeMap<UserId, UserEntryItem>>>,
    pub users_updates: Arc<RwLock<HashMap<u128, (UserId, UserEntryItem)>>>,
    pub stats: Arc<StatsAtomics>,
}
