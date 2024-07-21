use std::collections::BTreeMap;
use std::sync::Arc;
use crossbeam_skiplist::SkipMap;
use parking_lot::RwLock;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub torrents_map: Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>,
    pub peers_throttler: Arc<RwLock<BTreeMap<u128, (u64, u64)>>>, /* IP (can be IPv4 or IPV6), Timestamp and Counter */
    pub torrents_updates: Arc<SkipMap<InfoHash, i64>>,
    pub torrents_shadow: Arc<SkipMap<InfoHash, i64>>,
    pub stats: Arc<StatsAtomics>,
    pub torrents_whitelist: Arc<SkipMap<InfoHash, i64>>,
    pub torrents_blacklist: Arc<SkipMap<InfoHash, i64>>,
    pub keys: Arc<SkipMap<InfoHash, i64>>,
    pub users: Arc<SkipMap<UserId, UserEntryItem>>,
    pub users_updates: Arc<SkipMap<UserId, UserEntryItem>>,
    pub users_shadow: Arc<SkipMap<UserId, UserEntryItem>>,
    pub sqlx: DatabaseConnector,
}
