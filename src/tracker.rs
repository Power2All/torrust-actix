use chrono::Utc;
use scc::ebr::Arc;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::Mutex;

use crate::common::{InfoHash, PeerId, TorrentPeer};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;
use crate::tracker_objects::stats::Stats;
use crate::tracker_objects::torrents::TorrentEntryItem;
use crate::tracker_objects::users::UserEntryItem;

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub map_torrents: Arc<Mutex<BTreeMap<InfoHash, TorrentEntryItem>>>,
    pub map_peers: Arc<Mutex<BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>>>,
    pub updates: Arc<Mutex<HashMap<InfoHash, i64>>>,
    pub shadow: Arc<Mutex<HashMap<InfoHash, i64>>>,
    pub stats: Arc<Mutex<Stats>>,
    pub whitelist: Arc<Mutex<HashMap<InfoHash, i64>>>,
    pub blacklist: Arc<Mutex<HashMap<InfoHash, i64>>>,
    pub keys: Arc<Mutex<HashMap<InfoHash, i64>>>,
    pub users: Arc<Mutex<HashMap<String, UserEntryItem>>>,
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        TorrentTracker {
            config: config.clone(),
            map_torrents: Arc::new(Mutex::new(BTreeMap::new())),
            map_peers: Arc::new(Mutex::new(BTreeMap::new())),
            updates: Arc::new(Mutex::new(HashMap::new())),
            shadow: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(Stats {
                started: Utc::now().timestamp(),
                timestamp_run_save: 0,
                timestamp_run_timeout: 0,
                timestamp_run_console: 0,
                timestamp_run_keys_timeout: 0,
                torrents: 0,
                torrents_updates: 0,
                torrents_shadow: 0,
                users: 0,
                users_updates: 0,
                users_shadow: 0,
                maintenance_mode: 0,
                seeds: 0,
                peers: 0,
                completed: 0,
                whitelist_enabled: config.whitelist,
                whitelist: 0,
                blacklist_enabled: config.blacklist,
                blacklist: 0,
                keys_enabled: config.keys,
                keys: 0,
                tcp4_connections_handled: 0,
                tcp4_api_handled: 0,
                tcp4_announces_handled: 0,
                tcp4_scrapes_handled: 0,
                tcp6_connections_handled: 0,
                tcp6_api_handled: 0,
                tcp6_announces_handled: 0,
                tcp6_scrapes_handled: 0,
                udp4_connections_handled: 0,
                udp4_announces_handled: 0,
                udp4_scrapes_handled: 0,
                udp6_connections_handled: 0,
                udp6_announces_handled: 0,
                udp6_scrapes_handled: 0,
            })),
            whitelist: Arc::new(Mutex::new(HashMap::new())),
            blacklist: Arc::new(Mutex::new(HashMap::new())),
            keys: Arc::new(Mutex::new(HashMap::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
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