use scc::ebr::Arc;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::RwLock;

use crate::common::{InfoHash, PeerId, TorrentPeer};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;
use crate::tracker_channels::torrents_peers::TorrentEntryItem;

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct UserEntryItem {
//     pub uuid: String,
//     pub key: String,
//     pub uploaded: i64,
//     pub downloaded: i64,
//     pub completed: i64,
//     pub updated: i64,
//     pub active: i64,
// }
//
// impl UserEntryItem {
//     pub fn new() -> UserEntryItem {
//         UserEntryItem {
//             uuid: "".to_string(),
//             key: "".to_string(),
//             uploaded: 0,
//             downloaded: 0,
//             completed: 0,
//             updated: 0,
//             active: 0,
//         }
//     }
// }
//
// impl Default for UserEntryItem {
//     fn default() -> Self {
//         Self::new()
//     }
// }

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub torrents_peers_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub updates_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub shadow_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub whitelist_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub blacklist_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub keys_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub stats_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub map_torrents: Arc<RwLock<BTreeMap<InfoHash, TorrentEntryItem>>>,
    pub map_peers: Arc<RwLock<BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>>>,
    pub updates: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub shadow: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub whitelist: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub blacklist: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub keys: Arc<RwLock<HashMap<InfoHash, i64>>>,
    // pub users: Arc<RwLock<HashMap<String, UserEntryItem>>>,
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        let (torrents_peers_left, torrents_peers_right) = bichannel::channel();
        let (updates_left, updates_right) = bichannel::channel();
        let (shadow_left, shadow_right) = bichannel::channel();
        let (whitelist_left, whitelist_right) = bichannel::channel();
        let (blacklist_left, blacklist_right) = bichannel::channel();
        let (keys_left, keys_right) = bichannel::channel();
        let (stats_left, stats_right) = bichannel::channel();
        TorrentTracker {
            config: config.clone(),
            torrents_peers_channel: (torrents_peers_left, torrents_peers_right),
            updates_channel: (updates_left, updates_right),
            shadow_channel: (shadow_left, shadow_right),
            whitelist_channel: (whitelist_left, whitelist_right),
            blacklist_channel: (blacklist_left, blacklist_right),
            keys_channel: (keys_left, keys_right),
            stats_channel: (stats_left, stats_right),
            map_torrents: Arc::new(RwLock::new(BTreeMap::new())),
            map_peers: Arc::new(RwLock::new(BTreeMap::new())),
            updates: Arc::new(RwLock::new(HashMap::new())),
            shadow: Arc::new(RwLock::new(HashMap::new())),
            whitelist: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            keys: Arc::new(RwLock::new(HashMap::new())),
            // users: Arc::new(RwLock::new(HashMap::new())),
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
