use std::cell::Cell;
use chrono::{TimeZone, Utc};
use log::{debug, error, info};
use scc::ebr::Arc;
use serde::{Deserialize, Serialize};
use serde::de::value::MapDeserializer;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::ops::{Add, Deref};
use std::str::FromStr;
use std::sync::mpsc::{RecvError, SendError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use binascii::ConvertError;
use tokio::sync::RwLock;

use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StatsEvent {
    Torrents,
    TorrentsUpdates,
    TorrentsShadow,
    Users,
    UsersUpdates,
    UsersShadow,
    TimestampSave,
    TimestampTimeout,
    TimestampConsole,
    TimestampKeysTimeout,
    MaintenanceMode,
    Seeds,
    Peers,
    Completed,
    Whitelist,
    Blacklist,
    Key,
    Tcp4ConnectionsHandled,
    Tcp4ApiHandled,
    Tcp4AnnouncesHandled,
    Tcp4ScrapesHandled,
    Tcp6ConnectionsHandled,
    Tcp6ApiHandled,
    Tcp6AnnouncesHandled,
    Tcp6ScrapesHandled,
    Udp4ConnectionsHandled,
    Udp4AnnouncesHandled,
    Udp4ScrapesHandled,
    Udp6ConnectionsHandled,
    Udp6AnnouncesHandled,
    Udp6ScrapesHandled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Stats {
    pub started: i64,
    pub timestamp_run_save: i64,
    pub timestamp_run_timeout: i64,
    pub timestamp_run_console: i64,
    pub timestamp_run_keys_timeout: i64,
    pub torrents: i64,
    pub torrents_updates: i64,
    pub torrents_shadow: i64,
    pub users: i64,
    pub users_updates: i64,
    pub users_shadow: i64,
    pub maintenance_mode: i64,
    pub seeds: i64,
    pub peers: i64,
    pub completed: i64,
    pub whitelist_enabled: bool,
    pub whitelist: i64,
    pub blacklist_enabled: bool,
    pub blacklist: i64,
    pub keys_enabled: bool,
    pub keys: i64,
    pub tcp4_connections_handled: i64,
    pub tcp4_api_handled: i64,
    pub tcp4_announces_handled: i64,
    pub tcp4_scrapes_handled: i64,
    pub tcp6_connections_handled: i64,
    pub tcp6_api_handled: i64,
    pub tcp6_announces_handled: i64,
    pub tcp6_scrapes_handled: i64,
    pub udp4_connections_handled: i64,
    pub udp4_announces_handled: i64,
    pub udp4_scrapes_handled: i64,
    pub udp6_connections_handled: i64,
    pub udp6_announces_handled: i64,
    pub udp6_scrapes_handled: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentEntryItem {
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}

impl TorrentEntryItem {
    pub fn new() -> TorrentEntryItem {
        TorrentEntryItem {
            completed: 0,
            seeders: 0,
            leechers: 0,
        }
    }
}

impl Default for TorrentEntryItem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentEntry {
    #[serde(skip)]
    pub peers: BTreeMap<PeerId, TorrentPeer>,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}

impl TorrentEntry {
    pub fn new() -> TorrentEntry {
        TorrentEntry {
            peers: BTreeMap::new(),
            completed: 0,
            seeders: 0,
            leechers: 0,
        }
    }
}

impl Default for TorrentEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    pub uuid: String,
    pub key: String,
    pub uploaded: i64,
    pub downloaded: i64,
    pub completed: i64,
    pub updated: i64,
    pub active: i64,
}

impl UserEntryItem {
    pub fn new() -> UserEntryItem {
        UserEntryItem {
            uuid: "".to_string(),
            key: "".to_string(),
            uploaded: 0,
            downloaded: 0,
            completed: 0,
            updated: 0,
            active: 0,
        }
    }
}

impl Default for UserEntryItem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetTorrentsApi {
    pub info_hash: String,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetTorrentApi {
    pub info_hash: String,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
    pub peers: Vec<Value>,
}

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub torrents_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
    pub peers_channel: (bichannel::Channel<String, String>, bichannel::Channel<String, String>),
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
    pub users: Arc<RwLock<HashMap<String, UserEntryItem>>>,
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        let (torrents_left, torrents_right) = bichannel::channel();
        let (peers_left, peers_right) = bichannel::channel();
        let (updates_left, updates_right) = bichannel::channel();
        let (shadow_left, shadow_right) = bichannel::channel();
        let (whitelist_left, whitelist_right) = bichannel::channel();
        let (blacklist_left, blacklist_right) = bichannel::channel();
        let (keys_left, keys_right) = bichannel::channel();
        let (stats_left, stats_right) = bichannel::channel();
        TorrentTracker {
            config: config.clone(),
            torrents_channel: (torrents_left, torrents_right),
            peers_channel: (peers_left, peers_right),
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
