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

    /* === Channel: Torrents === */
    pub fn channel_torrents_init(&self)
    {
        let (channel_left, channel_right) = self.torrents_channel.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let mut torrents: BTreeMap<InfoHash, TorrentEntryItem> = BTreeMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "add_single" => {
                                let info_hash = match InfoHash::from_str(serde_json::from_value::<String>(data["data"]["info_hash"].clone()).unwrap().as_str()) {
                                    Ok(data) => { data }
                                    Err(error) => { channel_right.send(json!({"action": "error", "data": "error info_hash"}).to_string()).unwrap(); continue }
                                };
                                let entry = serde_json::from_value::<TorrentEntryItem>(data["data"]["entry"].clone()).unwrap();
                                let _ = torrents.insert(info_hash, entry);
                                channel_right.send(json!({
                                    "action": "add_single",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "add_multi" => {
                                let hashes: Result<Vec<(InfoHash, TorrentEntryItem)>, _> = serde_json::from_value(data["data"]["hashes"].clone());
                                for (info_hash, torrent_entry) in hashes.unwrap().iter() {
                                    let _ = torrents.insert(info_hash.clone(), torrent_entry.clone());
                                }
                                channel_right.send(json!({
                                    "action": "add_multi",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_single" => {
                                let info_hash = match InfoHash::from_str(serde_json::from_value::<String>(data["data"]["info_hash"].clone()).unwrap().as_str()) {
                                    Ok(data) => { data }
                                    Err(error) => { channel_right.send(json!({"action": "error", "data": "error info_hash"}).to_string()).unwrap(); continue }
                                };
                                let torrent = torrents.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "get_single",
                                    "data": torrent,
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi" => {
                                let mut torrentslist: Vec<(InfoHash, Option<TorrentEntry>)> = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for &info_hash in hashes.iter() {
                                    let torrent = match torrents.get(&info_hash) {
                                        None => { None }
                                        Some(torrent_entry) => {
                                            Some(TorrentEntry {
                                                peers: Default::default(),
                                                completed: torrent_entry.completed,
                                                seeders: torrent_entry.seeders,
                                                leechers: torrent_entry.leechers,
                                            })
                                        }
                                    };
                                    torrentslist.push((info_hash.clone(), torrent));
                                }
                                channel_right.send(json!({
                                    "action": "add_multi",
                                    "data": torrentslist,
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi_chunks" => {
                                let mut torrentslist: Vec<(InfoHash, i64)> = Vec::new();
                                let skip: u64 = serde_json::from_value::<u64>(data["data"]["skip"].clone()).unwrap();
                                let amount: u64 = serde_json::from_value::<u64>(data["data"]["amount"].clone()).unwrap();
                                let mut current_count: u64 = 0;
                                let mut handled_count: u64 = 0;
                                for (info_hash, entry) in torrents.iter() {
                                    if current_count < skip {
                                        current_count = current_count.add(1);
                                        continue;
                                    }
                                    if handled_count >= amount {
                                        break;
                                    }
                                    torrentslist.push((*info_hash, entry.completed));
                                    current_count = current_count.add(1);
                                    handled_count = handled_count.add(1);
                                }
                                channel_right.send(json!({
                                    "action": "get_multi_chunks",
                                    "data": torrentslist,
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_single" => {
                                let info_hash = match InfoHash::from_str(data["data"]["info_hash"].clone().to_string().as_str()) {
                                    Ok(data) => { data }
                                    Err(error) => { channel_right.send(json!({"action": "error", "data": "error info_hash"}).to_string()).unwrap(); continue }
                                };
                                let _ = torrents.remove(&info_hash);
                                channel_right.send(json!({
                                    "action": "get_single",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_multi" => {
                                let hashes: Result<Vec<InfoHash>, _> = serde_json::from_value(data["data"]["hashes"].clone());
                                let persistent = serde_json::from_value::<bool>(data["data"]["persistent"].clone()).unwrap();
                                for info_hash in hashes.unwrap().iter() {
                                    let _ = torrents.remove(info_hash);
                                }
                                channel_right.send(json!({
                                    "action": "delete_multi",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shutdown" => {
                                channel_right.send(json!({
                                    "action": "shutdown",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({
                                    "action": "error",
                                    "data": "unknown action",
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({
                            "action": "error",
                            "data": error.to_string(),
                            "torrent_count": torrents.len() as i64
                        }).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_torrents_request(&self, action: &str, data: Value) -> (Value, Value, Value)
    {
        let (channel_left, channel_right) = self.torrents_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone(), response_data["torrent_count"].clone())
    }

    pub async fn load_torrents(&self)
    {
        if let Ok(torrents) = self.sqlx.load_torrents().await {
            let mut torrent_count = 0i64;
            let mut completed_count = 0i64;

            for (info_hash, completed) in torrents.iter() {
                self.add_torrent(*info_hash, TorrentEntryItem {
                    completed: *completed,
                    seeders: 0,
                    leechers: 0,
                }, false).await;
                torrent_count += 1;
                completed_count += *completed;
            }

            info!("Loaded {} torrents with {} completes.", torrent_count, completed_count);
            self.update_stats(StatsEvent::Completed, completed_count).await;
        }
    }

    pub async fn save_torrents(&self) -> bool
    {
        let shadow = self.get_shadow().await;
        if self.sqlx.save_torrents(shadow).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntryItem, persistent: bool)
    {
        let (action, data, torrent_count) = self.channel_torrents_request(
            "add_single",
            json!({
                "info_hash": info_hash.clone(),
                "entry": torrent_entry.clone()
            })
        ).await;
        let torrents_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        self.update_stats(StatsEvent::Torrents, torrents_count).await;
        if persistent { self.add_update(info_hash, torrent_entry.completed).await; }
    }

    pub async fn add_torrents(&self, torrents: Vec<(InfoHash, TorrentEntryItem)>, persistent: bool)
    {
        let (_action, _data, torrent_count) = self.channel_torrents_request(
            "add_multi",
            json!({
                "hashes": torrents.clone()
            })
        ).await;
        let torrents_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        self.update_stats(StatsEvent::Torrents, torrents_count).await;
        if persistent {
            let mut updates = Vec::new();
            for (info_hash, torrent_entry) in torrents.iter() {
                updates.push((info_hash.clone(), torrent_entry.completed.clone()));
            }
            // self.add_updates(updates).await;
        }
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let (action, data, torrent_count) = self.channel_torrents_request(
            "get_single",
            json!({
                "info_hash": info_hash.clone()
            })
        ).await;
        let torrent_data = serde_json::from_value::<Option<TorrentEntryItem>>(data).unwrap();
        let torrent = match torrent_data {
            None => { None }
            Some(data) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.read().await;
                let peers = match peers_lock.get(&info_hash).cloned() {
                    None => { BTreeMap::new() }
                    Some(data) => { data }
                };
                drop(peers_lock);
                Some(TorrentEntry {
                    peers,
                    completed: data.completed,
                    seeders: data.seeders,
                    leechers: data.leechers,
                })
            }
        };
        torrent
    }

    pub async fn get_torrents(&self, hashes: Vec<InfoHash>) -> HashMap<InfoHash, Option<TorrentEntry>>
    {
        let mut return_torrents = HashMap::new();
        let (action, data, torrent_count) = self.channel_torrents_request(
            "get_multi",
            json!({
                "hashes": hashes.clone()
            })
        ).await;
        let torrents_data = serde_json::from_value::<Vec<(InfoHash, Option<TorrentEntry>)>>(data).unwrap();
        for (info_hash, torrent_entry) in torrents_data.iter() {
            return_torrents.insert(info_hash.clone(), match torrent_entry {
                None => { None }
                Some(data) => {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.read().await;
                    let peers = match peers_lock.get(info_hash).cloned() {
                        None => { BTreeMap::new() }
                        Some(data) => { data }
                    };
                    drop(peers_lock);
                    Some(TorrentEntry {
                        peers,
                        completed: data.completed,
                        seeders: data.seeders,
                        leechers: data.leechers,
                    })
                }
            });
        }
        return_torrents
    }

    pub async fn get_torrents_chunk(&self, skip: u64, amount: u64) -> HashMap<InfoHash, i64>
    {
        let mut return_torrents = HashMap::new();
        let (action, data, torrent_count) = self.channel_torrents_request(
            "get_multi_chunks",
            json!({
                "skip": skip.clone(),
                "amount": amount.clone()
            })
        ).await;
        let torrents_data = serde_json::from_value::<Vec<(InfoHash, Option<TorrentEntryItem>)>>(data).unwrap();
        for (info_hash, torrent_entry_item) in torrents_data.iter() {
            match torrent_entry_item {
                None => {}
                Some(torrent_entry) => {
                    return_torrents.insert(info_hash.clone(), torrent_entry.completed.clone());
                }
            }
        }
        return_torrents
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool)
    {
        let mut removed_torrent = false;
        let mut remove_seeders = 0i64;
        let mut remove_leechers = 0i64;

        let torrents_arc = self.map_torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let torrent_option = torrents_lock.get(&info_hash);
        match torrent_option {
            None => {}
            Some(data) => {
                removed_torrent = true;
                remove_seeders -= data.seeders;
                remove_leechers -= data.leechers;
                torrents_lock.remove(&info_hash);
            }
        }
        drop(torrents_lock);

        let peers_arc = self.map_peers.clone();
        let mut peers_lock = peers_arc.write().await;
        peers_lock.remove(&info_hash);
        drop(peers_lock);

        if persistent {
            self.remove_update(info_hash).await;
            self.remove_shadow(info_hash).await;
        }

        if removed_torrent { self.update_stats(StatsEvent::Torrents, -1).await; }
        if remove_seeders != 0 { self.update_stats(StatsEvent::Seeds, remove_seeders).await; }
        if remove_leechers != 0 { self.update_stats(StatsEvent::Peers, remove_leechers).await; }
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>, persistent: bool)
    {
        for info_hash in hashes.iter() {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.read().await;
            let torrent_option = torrents_lock.get(info_hash).cloned();
            drop(torrents_lock);

            if torrent_option.is_some() {
                self.remove_torrent(*info_hash, persistent).await;
            }
        }
    }

    pub fn channel_peers_init(&self)
    {
        let (channel_left, channel_right) = self.peers_channel.clone();
        tokio::spawn(async move {
            let mut peers: BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>> = BTreeMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_peers_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.peers_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub fn channel_updates_init(&self)
    {
        let (channel_left, channel_right) = self.updates_channel.clone();
        tokio::spawn(async move {
            let mut updates: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_updates_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.updates_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub fn channel_shadow_init(&self)
    {
        let (channel_left, channel_right) = self.shadow_channel.clone();
        tokio::spawn(async move {
            let mut shadow: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_shadow_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.shadow_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub fn channel_whitelist_init(&self)
    {
        let (channel_left, channel_right) = self.whitelist_channel.clone();
        tokio::spawn(async move {
            let mut whitelist: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_whitelist_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.whitelist_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub fn channel_blacklist_init(&self)
    {
        let (channel_left, channel_right) = self.blacklist_channel.clone();
        tokio::spawn(async move {
            let mut blacklist: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_blacklist_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.blacklist_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub fn channel_keys_init(&self)
    {
        let (channel_left, channel_right) = self.keys_channel.clone();
        tokio::spawn(async move {
            let mut keys: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_keys_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.keys_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    /* === Channel: Statistics === */
    pub fn channel_stats_init(&self)
    {
        let (channel_left, channel_right) = self.stats_channel.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let mut stats: Stats = Stats {
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
            };

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "get" => {
                                channel_right.send(json!({"action": "get", "data": stats}).to_string()).unwrap();
                            }
                            "set" => {
                                let event: StatsEvent = serde_json::from_value::<StatsEvent>(data["data"]["event"].clone()).unwrap();
                                let value: i64 = serde_json::from_value::<i64>(data["data"]["value"].clone()).unwrap();
                                match event {
                                    StatsEvent::Torrents => { stats.torrents = value; }
                                    StatsEvent::TorrentsUpdates => { stats.torrents_updates = value; }
                                    StatsEvent::TorrentsShadow => { stats.torrents_shadow = value; }
                                    StatsEvent::Users => { stats.users = value; }
                                    StatsEvent::UsersUpdates => { stats.users_updates = value; }
                                    StatsEvent::UsersShadow => { stats.users_shadow = value; }
                                    StatsEvent::TimestampSave => { stats.timestamp_run_save = value; }
                                    StatsEvent::TimestampTimeout => { stats.timestamp_run_timeout = value; }
                                    StatsEvent::TimestampConsole => { stats.timestamp_run_console = value; }
                                    StatsEvent::TimestampKeysTimeout => { stats.timestamp_run_keys_timeout = value; }
                                    StatsEvent::MaintenanceMode => { stats.maintenance_mode = value; }
                                    StatsEvent::Seeds => { stats.seeds = value; }
                                    StatsEvent::Peers => { stats.peers = value; }
                                    StatsEvent::Completed => { stats.completed = value; }
                                    StatsEvent::Whitelist => { stats.whitelist = value; }
                                    StatsEvent::Blacklist => { stats.blacklist = value; }
                                    StatsEvent::Key => { stats.keys = value; }
                                    StatsEvent::Tcp4ConnectionsHandled => { stats.tcp4_connections_handled = value; }
                                    StatsEvent::Tcp4ApiHandled => { stats.tcp4_api_handled = value; }
                                    StatsEvent::Tcp4AnnouncesHandled => { stats.tcp4_announces_handled = value; }
                                    StatsEvent::Tcp4ScrapesHandled => { stats.tcp4_scrapes_handled = value; }
                                    StatsEvent::Tcp6ConnectionsHandled => { stats.tcp6_connections_handled = value; }
                                    StatsEvent::Tcp6ApiHandled => { stats.tcp6_api_handled = value; }
                                    StatsEvent::Tcp6AnnouncesHandled => { stats.tcp6_announces_handled = value; }
                                    StatsEvent::Tcp6ScrapesHandled => { stats.tcp6_scrapes_handled = value; }
                                    StatsEvent::Udp4ConnectionsHandled => { stats.udp4_connections_handled = value; }
                                    StatsEvent::Udp4AnnouncesHandled => { stats.udp4_announces_handled = value; }
                                    StatsEvent::Udp4ScrapesHandled => { stats.udp4_scrapes_handled = value; }
                                    StatsEvent::Udp6ConnectionsHandled => { stats.udp6_connections_handled = value; }
                                    StatsEvent::Udp6AnnouncesHandled => { stats.udp6_announces_handled = value; }
                                    StatsEvent::Udp6ScrapesHandled => { stats.udp6_scrapes_handled = value; }
                                }
                                channel_right.send(json!({"action": "set", "data": stats}).to_string()).unwrap();
                            }
                            "update" => {
                                let event: StatsEvent = serde_json::from_value::<StatsEvent>(data["data"]["event"].clone()).unwrap();
                                let value: i64 = serde_json::from_value::<i64>(data["data"]["value"].clone()).unwrap();
                                match event {
                                    StatsEvent::Torrents => { stats.torrents += value; }
                                    StatsEvent::TorrentsUpdates => { stats.torrents_updates += value; }
                                    StatsEvent::TorrentsShadow => { stats.torrents_shadow += value; }
                                    StatsEvent::Users => { stats.users += value; }
                                    StatsEvent::UsersUpdates => { stats.users_updates += value; }
                                    StatsEvent::UsersShadow => { stats.users_shadow += value; }
                                    StatsEvent::TimestampSave => { stats.timestamp_run_save += value; }
                                    StatsEvent::TimestampTimeout => { stats.timestamp_run_timeout += value; }
                                    StatsEvent::TimestampConsole => { stats.timestamp_run_console += value; }
                                    StatsEvent::TimestampKeysTimeout => { stats.timestamp_run_keys_timeout += value; }
                                    StatsEvent::MaintenanceMode => { stats.maintenance_mode += value; }
                                    StatsEvent::Seeds => { stats.seeds += value; }
                                    StatsEvent::Peers => { stats.peers += value; }
                                    StatsEvent::Completed => { stats.completed += value; }
                                    StatsEvent::Whitelist => { stats.whitelist += value; }
                                    StatsEvent::Blacklist => { stats.blacklist += value; }
                                    StatsEvent::Key => { stats.keys += value; }
                                    StatsEvent::Tcp4ConnectionsHandled => { stats.tcp4_connections_handled += value; }
                                    StatsEvent::Tcp4ApiHandled => { stats.tcp4_api_handled += value; }
                                    StatsEvent::Tcp4AnnouncesHandled => { stats.tcp4_announces_handled += value; }
                                    StatsEvent::Tcp4ScrapesHandled => { stats.tcp4_scrapes_handled += value; }
                                    StatsEvent::Tcp6ConnectionsHandled => { stats.tcp6_connections_handled += value; }
                                    StatsEvent::Tcp6ApiHandled => { stats.tcp6_api_handled += value; }
                                    StatsEvent::Tcp6AnnouncesHandled => { stats.tcp6_announces_handled += value; }
                                    StatsEvent::Tcp6ScrapesHandled => { stats.tcp6_scrapes_handled += value; }
                                    StatsEvent::Udp4ConnectionsHandled => { stats.udp4_connections_handled += value; }
                                    StatsEvent::Udp4AnnouncesHandled => { stats.udp4_announces_handled += value; }
                                    StatsEvent::Udp4ScrapesHandled => { stats.udp4_scrapes_handled += value; }
                                    StatsEvent::Udp6ConnectionsHandled => { stats.udp6_connections_handled += value; }
                                    StatsEvent::Udp6AnnouncesHandled => { stats.udp6_announces_handled += value; }
                                    StatsEvent::Udp6ScrapesHandled => { stats.udp6_scrapes_handled += value; }
                                };
                                channel_right.send(json!({"action": "update", "data": stats}).to_string()).unwrap();
                            }
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_stats_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.stats_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub async fn get_stats(&self) -> Stats
    {
        let (action, data) = self.channel_stats_request("get", json!({})).await;
        let stats = serde_json::from_value::<Stats>(data).unwrap();
        stats
    }

    pub async fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let (action, data) = self.channel_stats_request("update", json!({
            "event": event,
            "value": value
        })).await;
        let stats = serde_json::from_value::<Stats>(data).unwrap();
        stats
    }

    pub async fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let (action, data) = self.channel_stats_request("set", json!({
            "event": event,
            "value": value
        })).await;
        let stats = serde_json::from_value::<Stats>(data).unwrap();
        stats
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

    pub async fn load_whitelists(&self)
    {
        if let Ok(whitelists) = self.sqlx.load_whitelist().await {
            let mut whitelist_count = 0i64;

            for info_hash in whitelists.iter() {
                self.add_whitelist(*info_hash, true).await;
                whitelist_count += 1;
            }

            info!("Loaded {} whitelists.", whitelist_count);
        }
    }

    pub async fn load_blacklists(&self)
    {
        if let Ok(blacklists) = self.sqlx.load_blacklist().await {
            let mut blacklist_count = 0i64;

            for info_hash in blacklists.iter() {
                self.add_blacklist(*info_hash, true).await;
                blacklist_count += 1;
            }

            info!("Loaded {} blacklists.", blacklist_count);
        }
    }

    pub async fn load_keys(&self)
    {
        if let Ok(keys) = self.sqlx.load_keys().await {
            let mut keys_count = 0i64;

            for (hash, timeout) in keys.iter() {
                self.add_key_raw(*hash, *timeout).await;
                keys_count += 1;
            }

            info!("Loaded {} keys.", keys_count);
        }
    }

    pub async fn save_whitelists(&self) -> bool
    {
        let whitelist = self.get_whitelist().await;
        if self.sqlx.save_whitelist(whitelist.clone()).await.is_ok() {
            for (info_hash, value) in whitelist.iter() {
                if value == &0 {
                    self.remove_whitelist(*info_hash).await;
                }
                if value == &2 {
                    self.add_whitelist(*info_hash, true).await;
                }
            }
            return true;
        }
        false
    }

    pub async fn save_blacklists(&self) -> bool
    {
        let blacklist = self.get_blacklist().await;
        if self.sqlx.save_blacklist(blacklist).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn save_keys(&self) -> bool
    {
        let keys = self.get_keys().await;
        if self.sqlx.save_keys(keys).await.is_ok() {
            return true;
        }
        false
    }

    /* === Peers === */
    pub async fn add_peer(&self, info_hash: InfoHash, peer_id: PeerId, peer_entry: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let mut added_seeder = false;
        let mut added_leecher = false;
        let mut removed_seeder = false;
        let mut removed_leecher = false;
        let mut completed_applied = false;

        let torrents_arc = self.map_torrents.clone();
        let torrents_lock = torrents_arc.read().await;
        let torrent_input = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.read().await;
                let peer = peers_lock.get(&info_hash).cloned();
                drop(peers_lock);

                let mut peers = match peer {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers }
                };

                match peers.get(&peer_id).cloned() {
                    None => {
                        if peer_entry.left == NumberOfBytes(0) {
                            data_torrent.seeders += 1;
                            added_seeder = true;
                            if completed {
                                data_torrent.completed += 1;
                                completed_applied = true;
                            }
                        } else {
                            data_torrent.leechers += 1;
                            added_leecher = true;
                        }
                        let _ = peers.insert(peer_id, peer_entry);
                    }
                    Some(data_peer) => {
                        if data_peer.left == NumberOfBytes(0) && peer_entry.left != NumberOfBytes(0) {
                            data_torrent.seeders -= 1;
                            data_torrent.leechers += 1;
                            removed_seeder = true;
                            added_leecher = true;
                        } else if data_peer.left != NumberOfBytes(0) && peer_entry.left == NumberOfBytes(0) {
                            data_torrent.seeders += 1;
                            data_torrent.leechers -= 1;
                            added_seeder = true;
                            removed_leecher = true;
                            if completed {
                                data_torrent.completed += 1;
                                completed_applied = true;
                            }
                        }
                        let _ = peers.insert(peer_id, peer_entry);
                    }
                };

                let torrents_arc = self.map_torrents.clone();
                let mut torrents_lock = torrents_arc.write().await;
                torrents_lock.insert(info_hash, data_torrent.clone());
                drop(torrents_lock);

                let peers_arc = self.map_peers.clone();
                let mut peers_lock = peers_arc.write().await;
                peers_lock.insert(info_hash, peers.clone());
                drop(peers_lock);

                TorrentEntry {
                    peers,
                    completed: data_torrent.completed,
                    seeders: data_torrent.seeders,
                    leechers: data_torrent.leechers,
                }
            }
        };

        if persistent && completed {
            self.add_update(
                info_hash,
                torrent.completed,
            ).await;
        }

        if added_seeder { self.update_stats(StatsEvent::Seeds, 1).await; }
        if added_leecher { self.update_stats(StatsEvent::Peers, 1).await; }
        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
        if completed_applied { self.update_stats(StatsEvent::Completed, 1).await; }

        torrent
    }

    pub async fn remove_peer(&self, info_hash: InfoHash, peer_id: PeerId, _persistent: bool) -> TorrentEntry
    {
        let mut removed_seeder = false;
        let mut removed_leecher = false;

        let torrents_arc = self.map_torrents.clone();
        let torrents_lock = torrents_arc.read().await;
        let torrent_input = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.read().await;
                let peer = peers_lock.get(&info_hash).cloned();
                drop(peers_lock);

                let mut peers = match peer {
                    None => { BTreeMap::new() }
                    Some(data_peers) => { data_peers }
                };
                let peer_option = peers.get(&peer_id);
                if peer_option.is_some() {
                    let peer = *peer_option.unwrap();
                    if peer.left == NumberOfBytes(0) {
                        peers.remove(&peer_id);
                        data_torrent.seeders -= 1;
                        removed_seeder = true;
                    } else {
                        peers.remove(&peer_id);
                        data_torrent.leechers -= 1;
                        removed_leecher = true;
                    }
                }

                let torrents_arc = self.map_torrents.clone();
                let mut torrents_lock = torrents_arc.write().await;
                torrents_lock.insert(info_hash, data_torrent.clone());
                drop(torrents_lock);

                if peers.is_empty() {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.write().await;
                    peers_lock.remove(&info_hash);
                    drop(peers_lock);
                } else {
                    let peers_arc = self.map_peers.clone();
                    let mut peers_lock = peers_arc.write().await;
                    peers_lock.insert(info_hash, peers.clone());
                    drop(peers_lock);
                }

                TorrentEntry {
                    peers,
                    completed: data_torrent.completed,
                    seeders: data_torrent.seeders,
                    leechers: data_torrent.leechers,
                }
            }
        };

        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }

        torrent
    }

    pub async fn remove_peers(&self, peers: Vec<(InfoHash, PeerId)>, _persistent: bool) -> HashMap<InfoHash, TorrentEntry>
    {
        let mut removed_seeder = 0i64;
        let mut removed_leecher = 0i64;
        let mut return_torrententries = HashMap::new();

        for (info_hash, peer_id) in peers.iter() {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.read().await;
            let torrent = torrents_lock.get(info_hash).cloned();
            drop(torrents_lock);

            return_torrententries.insert(*info_hash, match torrent {
                None => { TorrentEntry::new() }
                Some(mut data_torrent) => {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.read().await;
                    let peer = peers_lock.get(info_hash).cloned();
                    drop(peers_lock);

                    let mut peers = match peer {
                        None => { BTreeMap::new() }
                        Some(data_peers) => { data_peers }
                    };

                    let peer_option = peers.get(peer_id);
                    if peer_option.is_some() {
                        let peer = *peer_option.unwrap();
                        if peer.left == NumberOfBytes(0) {
                            peers.remove(peer_id);
                            data_torrent.seeders -= 1;
                            removed_seeder -= 1;
                        } else {
                            peers.remove(peer_id);
                            data_torrent.leechers -= 1;
                            removed_leecher -= 1;
                        }
                    }

                    let torrents_arc = self.map_torrents.clone();
                    let mut torrents_lock = torrents_arc.write().await;
                    torrents_lock.insert(*info_hash, data_torrent.clone());
                    drop(torrents_lock);

                    if peers.is_empty() {
                        let peers_arc = self.map_peers.clone();
                        let mut peers_lock = peers_arc.write().await;
                        peers_lock.remove(info_hash);
                        drop(peers_lock);
                    } else {
                        let peers_arc = self.map_peers.clone();
                        let mut peers_lock = peers_arc.write().await;
                        peers_lock.insert(*info_hash, peers.clone());
                        drop(peers_lock);
                    }

                    TorrentEntry {
                        peers,
                        completed: data_torrent.completed,
                        seeders: data_torrent.seeders,
                        leechers: data_torrent.leechers,
                    }
                }
            });
        }

        if removed_seeder != 0 { self.update_stats(StatsEvent::Seeds, removed_seeder).await; }
        if removed_leecher != 0 { self.update_stats(StatsEvent::Peers, removed_leecher).await; }

        return_torrententries
    }

    pub async fn clean_peers(&self, peer_timeout: Duration)
    {
        // Cleaning up peers in chunks, to prevent slow behavior.
        let mut start: usize = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        let mut removed_peers = 0u64;

        loop {
            info!("[PEERS] Scanning peers {} to {}", start, (start + size));

            let peers_arc = self.map_peers.clone();
            let peers_lock = peers_arc.read().await;
            let mut torrent_index = vec![];
            for (info_hash, _) in peers_lock.iter().skip(start) {
                torrent_index.push(*info_hash);
                if torrent_index.len() == size {
                    break;
                }
            }
            drop(peers_lock);

            let mut peers = vec![];
            let torrents = self.get_torrents(torrent_index.clone()).await;
            for (info_hash, torrent_entry) in torrents.iter() {
                if torrent_entry.is_some() {
                    let torrent = torrent_entry.clone().unwrap().clone();
                    for (peer_id, torrent_peer) in torrent.peers.iter() {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            peers.push((*info_hash, *peer_id));
                        }
                    }
                } else {
                    continue;
                }
            }
            removed_peers += peers.len() as u64;
            let _ = self.remove_peers(peers, self.config.clone().persistence).await;

            if torrent_index.len() != size {
                break;
            }

            start += size;
        }
        info!("[PEERS] Removed {} peers", removed_peers);
    }

    /* === Updates === */
    pub async fn add_update(&self, info_hash: InfoHash, completed: i64)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.write().await;
        updates_lock.insert(info_hash, completed);
        let update_count = updates_lock.len();
        drop(updates_lock);

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn add_updates(&self, updates: HashMap<InfoHash, i64>)
    {
        let mut update_count = 0;

        for (info_hash, completed) in updates.iter() {
            let updates_arc = self.updates.clone();
            let mut updates_lock = updates_arc.write().await;
            updates_lock.insert(*info_hash, *completed);
            update_count = updates_lock.len();
            drop(updates_lock);
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn get_update(&self) -> HashMap<InfoHash, i64>
    {
        let updates_arc = self.updates.clone();
        let updates_lock = updates_arc.read().await;
        let updates = updates_lock.clone();
        drop(updates_lock);

        updates
    }

    pub async fn remove_update(&self, info_hash: InfoHash)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.write().await;
        updates_lock.remove(&info_hash);
        let update_count = updates_lock.len();
        drop(updates_lock);

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn remove_updates(&self, hashes: Vec<InfoHash>)
    {
        let mut update_count = 0;

        for info_hash in hashes.iter() {
            let updates_arc = self.updates.clone();
            let mut updates_lock = updates_arc.write().await;
            updates_lock.remove(info_hash);
            update_count = updates_lock.len();
            drop(updates_lock);
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn transfer_updates_to_shadow(&self)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.write().await;
        let updates = updates_lock.clone();
        updates_lock.clear();
        drop(updates_lock);

        for (info_hash, completed) in updates.iter() {
            self.add_shadow(*info_hash, *completed).await;
        }

        self.set_stats(StatsEvent::TorrentsUpdates, 0).await;
    }

    /* === Shadow === */
    pub async fn add_shadow(&self, info_hash: InfoHash, completed: i64)
    {
        let shadow_arc = self.shadow.clone();
        let mut shadow_lock = shadow_arc.write().await;
        shadow_lock.insert(info_hash, completed);
        let shadow_count = shadow_lock.len();
        drop(shadow_lock);

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_shadow(&self, info_hash: InfoHash)
    {
        let shadow_arc = self.shadow.clone();
        let mut shadow_lock = shadow_arc.write().await;
        shadow_lock.remove(&info_hash);
        let shadow_count = shadow_lock.len();
        drop(shadow_lock);

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_shadows(&self, hashes: Vec<InfoHash>)
    {
        let mut shadow_count = 0;

        for info_hash in hashes.iter() {
            let shadow_arc = self.shadow.clone();
            let mut shadow_lock = shadow_arc.write().await;
            shadow_lock.remove(info_hash);
            shadow_count = shadow_lock.len();
            drop(shadow_lock);
        }

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn get_shadow(&self) -> HashMap<InfoHash, i64>
    {
        let shadow_arc = self.shadow.clone();
        let shadow_lock = shadow_arc.read().await;
        let shadow = shadow_lock.clone();
        drop(shadow_lock);

        shadow
    }

    pub async fn clear_shadow(&self)
    {
        let shadow_arc = self.shadow.clone();
        let mut shadow_lock = shadow_arc.write().await;
        shadow_lock.clear();
        drop(shadow_lock);

        self.set_stats(StatsEvent::TorrentsShadow, 0).await;
    }

    /* === Whitelist === */
    pub async fn add_whitelist(&self, info_hash: InfoHash, on_load: bool)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        if on_load {
            whitelist_lock.insert(info_hash, 1i64);
        } else {
            whitelist_lock.insert(info_hash, 2i64);
        }
        drop(whitelist_lock);

        self.update_stats(StatsEvent::Whitelist, 1).await;
    }

    pub async fn get_whitelist(&self) -> HashMap<InfoHash, i64>
    {
        let mut return_list = HashMap::new();

        let whitelist_arc = self.whitelist.clone();
        let whitelist_lock = whitelist_arc.read().await;
        for (info_hash, value) in whitelist_lock.iter() {
            return_list.insert(*info_hash, *value);
        }
        drop(whitelist_lock);

        return_list
    }

    pub async fn remove_flag_whitelist(&self, info_hash: InfoHash)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        if whitelist_lock.get(&info_hash).is_some() {
            whitelist_lock.insert(info_hash, 0i64);
        }
        let whitelists = whitelist_lock.clone();
        drop(whitelist_lock);

        let mut whitelist_count = 0i64;
        for (_, value) in whitelists.iter() {
            if value == &1i64 {
                whitelist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn remove_whitelist(&self, info_hash: InfoHash)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        whitelist_lock.remove(&info_hash);
        let whitelists = whitelist_lock.clone();
        drop(whitelist_lock);

        let mut whitelist_count = 0i64;
        for (_, value) in whitelists.iter() {
            if value == &1 {
                whitelist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let whitelist_arc = self.whitelist.clone();
        let whitelist_lock = whitelist_arc.read().await;
        let whitelist = whitelist_lock.get(&info_hash).cloned();
        drop(whitelist_lock);

        if whitelist.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_whitelist(&self)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        whitelist_lock.clear();
        drop(whitelist_lock);

        self.set_stats(StatsEvent::Whitelist, 0).await;
    }

    /* === Blacklist === */
    pub async fn add_blacklist(&self, info_hash: InfoHash, on_load: bool)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        if on_load {
            blacklist_lock.insert(info_hash, 1i64);
        } else {
            blacklist_lock.insert(info_hash, 2i64);
        }
        drop(blacklist_lock);

        self.update_stats(StatsEvent::Blacklist, 1).await;
    }

    pub async fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let mut return_list = vec![];

        let blacklist_arc = self.blacklist.clone();
        let blacklist_lock = blacklist_arc.read().await;
        for (info_hash, _) in blacklist_lock.iter() {
            return_list.push(*info_hash);
        }
        drop(blacklist_lock);

        return_list
    }

    pub async fn remove_flag_blacklist(&self, info_hash: InfoHash)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        if blacklist_lock.get(&info_hash).is_some() {
            blacklist_lock.insert(info_hash, 0i64);
        }
        let blacklists = blacklist_lock.clone();
        drop(blacklist_lock);

        let mut blacklist_count = 0i64;
        for (_, value) in blacklists.iter() {
            if value == &1i64 {
                blacklist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Blacklist, blacklist_count).await;
    }

    pub async fn remove_blacklist(&self, info_hash: InfoHash)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        blacklist_lock.remove(&info_hash);
        let blacklists = blacklist_lock.clone();
        drop(blacklist_lock);

        let mut blacklist_count = 0i64;
        for (_, value) in blacklists.iter() {
            if value == &1 {
                blacklist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Blacklist, blacklist_count).await;
    }

    pub async fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let blacklist_arc = self.blacklist.clone();
        let blacklist_lock = blacklist_arc.read().await;
        let blacklist = blacklist_lock.get(&info_hash).cloned();
        drop(blacklist_lock);

        if blacklist.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_blacklist(&self)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        blacklist_lock.clear();
        drop(blacklist_lock);

        self.set_stats(StatsEvent::Blacklist, 0).await;
    }

    /* === Keys === */
    pub async fn add_key(&self, hash: InfoHash, timeout: i64)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_unix = timestamp.as_secs() as i64 + timeout;
        keys_lock.insert(hash, timeout_unix);
        drop(keys_lock);

        self.update_stats(StatsEvent::Key, 1).await;
    }

    pub async fn add_key_raw(&self, hash: InfoHash, timeout: i64)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        let time = SystemTime::from(Utc.timestamp_opt(timeout, 0).unwrap());
        match time.duration_since(SystemTime::now()) {
            Ok(_) => {
                keys_lock.insert(hash, timeout);
            }
            Err(_) => {
                drop(keys_lock);
                return;
            }
        }
        drop(keys_lock);

        self.update_stats(StatsEvent::Key, 1).await;
    }

    pub async fn get_keys(&self) -> Vec<(InfoHash, i64)>
    {
        let keys_arc = self.keys.clone();
        let keys_lock = keys_arc.read().await;
        let keys = keys_lock.clone();
        drop(keys_lock);

        let mut return_list = vec![];
        for (hash, timeout) in keys.iter() {
            return_list.push((*hash, *timeout));
        }

        return_list
    }

    pub async fn remove_key(&self, hash: InfoHash)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        keys_lock.remove(&hash);
        let key_count = keys_lock.len();
        drop(keys_lock);

        self.set_stats(StatsEvent::Key, key_count as i64).await;
    }

    pub async fn check_key(&self, hash: InfoHash) -> bool
    {
        let keys_arc = self.keys.clone();
        let keys_lock = keys_arc.read().await;
        let key = keys_lock.get(&hash).cloned();
        drop(keys_lock);

        if key.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_keys(&self)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        keys_lock.clear();
        drop(keys_lock);

        self.set_stats(StatsEvent::Key, 0).await;
    }

    pub async fn clean_keys(&self)
    {
        let keys_arc = self.keys.clone();
        let keys_lock = keys_arc.read().await;
        let keys = keys_lock.clone();
        drop(keys_lock);

        let mut keys_index = vec![];
        for (hash, timeout) in keys.iter() {
            keys_index.push((*hash, *timeout));
        }

        for (hash, timeout) in keys_index.iter() {
            if *timeout != 0 {
                let time = SystemTime::from(Utc.timestamp_opt(*timeout, 0).unwrap());
                match time.duration_since(SystemTime::now()) {
                    Ok(_) => {}
                    Err(_) => {
                        self.remove_key(*hash).await;
                    }
                }
            }
        }
    }
}
