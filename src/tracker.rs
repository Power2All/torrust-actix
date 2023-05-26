use scc::ebr::Arc;

use crate::common::{channel, Channel};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;

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
    pub torrents_peers_channel: (Channel<String, String>, Channel<String, String>),
    pub updates_shadow_channel: (Channel<String, String>, Channel<String, String>),
    pub whitelist_blacklist_keys_channel: (Channel<String, String>, Channel<String, String>),
    pub stats_channel: (Channel<String, String>, Channel<String, String>),
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        let (torrents_peers_left, torrents_peers_right) = channel();
        let (updates_shadow_left, updates_shadow_right) = channel();
        let (whitelist_blacklist_keys_left, whitelist_blacklist_keys_right) = channel();
        let (stats_left, stats_right) = channel();
        TorrentTracker {
            config: config.clone(),
            torrents_peers_channel: (torrents_peers_left, torrents_peers_right),
            updates_shadow_channel: (updates_shadow_left, updates_shadow_right),
            whitelist_blacklist_keys_channel: (whitelist_blacklist_keys_left, whitelist_blacklist_keys_right),
            stats_channel: (stats_left, stats_right),
            sqlx: DatabaseConnector::new(config.clone()).await,
        }
    }
}
