use chrono::Utc;
use scc::ebr::Arc;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicI64};
use async_std::sync::RwLock;

use crate::common::{InfoHash, PeerId, TorrentPeer};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;
use crate::tracker_objects::stats::StatsAtomics;
use crate::tracker_objects::torrents::TorrentEntryItem;
use crate::tracker_objects::users::UserEntryItem;

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub map_torrents: Arc<RwLock<BTreeMap<InfoHash, TorrentEntryItem>>>,
    pub map_peers: Arc<RwLock<BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>>>,
    pub updates: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub shadow: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub stats: Arc<StatsAtomics>,
    pub whitelist: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub blacklist: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub keys: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub users: Arc<RwLock<HashMap<String, UserEntryItem>>>,
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        TorrentTracker {
            config: config.clone(),
            map_torrents: Arc::new(RwLock::new(BTreeMap::new())),
            map_peers: Arc::new(RwLock::new(BTreeMap::new())),
            updates: Arc::new(RwLock::new(HashMap::new())),
            shadow: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(StatsAtomics {
                started: AtomicI64::new(Utc::now().timestamp()),
                timestamp_run_save: AtomicI64::new(0),
                timestamp_run_timeout: AtomicI64::new(0),
                timestamp_run_console: AtomicI64::new(0),
                timestamp_run_keys_timeout: AtomicI64::new(0),
                torrents: AtomicI64::new(0),
                torrents_updates: AtomicI64::new(0),
                torrents_shadow: AtomicI64::new(0),
                users: AtomicI64::new(0),
                users_updates: AtomicI64::new(0),
                users_shadow: AtomicI64::new(0),
                maintenance_mode: AtomicI64::new(0),
                seeds: AtomicI64::new(0),
                peers: AtomicI64::new(0),
                completed: AtomicI64::new(0),
                whitelist_enabled: AtomicBool::new(config.whitelist),
                whitelist: AtomicI64::new(0),
                blacklist_enabled: AtomicBool::new(config.blacklist),
                blacklist: AtomicI64::new(0),
                keys_enabled: AtomicBool::new(config.keys),
                keys: AtomicI64::new(0),
                tcp4_connections_handled: AtomicI64::new(0),
                tcp4_api_handled: AtomicI64::new(0),
                tcp4_announces_handled: AtomicI64::new(0),
                tcp4_scrapes_handled: AtomicI64::new(0),
                tcp6_connections_handled: AtomicI64::new(0),
                tcp6_api_handled: AtomicI64::new(0),
                tcp6_announces_handled: AtomicI64::new(0),
                tcp6_scrapes_handled: AtomicI64::new(0),
                udp4_connections_handled: AtomicI64::new(0),
                udp4_announces_handled: AtomicI64::new(0),
                udp4_scrapes_handled: AtomicI64::new(0),
                udp6_connections_handled: AtomicI64::new(0),
                udp6_announces_handled: AtomicI64::new(0),
                udp6_scrapes_handled: AtomicI64::new(0),
            }),
            whitelist: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            keys: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            sqlx: DatabaseConnector::new(config.clone()).await,
        }
    }

    // pub async fn load_users(&self)
    // {
    //     if let Ok(users) = self.sqlx.load_users().await {
    //         let mut user_count = 0i64;
    //
    //         for (info_hash, completed) in torrents.iter() {
    //             self.add_torrent(*info_hash, TorrentEntryItem {
    //                 completed: *completed,
    //                 seeders: 0,
    //                 leechers: 0,
    //             }, false).await;
    //             torrent_count += 1;
    //             completed_count += *completed;
    //         }
    //
    //         info!("Loaded {} torrents with {} completes.", torrent_count, completed_count);
    //         self.update_stats(StatsEvent::Completed, completed_count).await;
    //     }
    // }

}