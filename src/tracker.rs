use chrono::{TimeZone, Utc};
use log::info;
use scc::ebr::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;

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

#[derive(Serialize, Deserialize, Clone)]
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
    pub map_torrents: Arc<RwLock<BTreeMap<InfoHash, TorrentEntryItem>>>,
    pub map_peers: Arc<RwLock<BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>>>,
    pub updates: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub shadow: Arc<RwLock<HashMap<InfoHash, i64>>>,
    pub stats: Arc<RwLock<Stats>>,
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
            stats: Arc::new(RwLock::new(Stats {
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
            whitelist: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            keys: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            sqlx: DatabaseConnector::new(config.clone()).await,
        }
    }

    /* === Statistics === */
    pub async fn get_stats(&self) -> Stats
    {
        let stats_arc = self.stats.clone();
        let stats_lock = stats_arc.write().await;
        let stats = stats_lock.clone();
        drop(stats_lock);

        stats
    }

    pub async fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let stats_arc = self.stats.clone();
        let mut stats_lock = stats_arc.write().await;
        match event {
            StatsEvent::Torrents => { stats_lock.torrents += value; }
            StatsEvent::TorrentsUpdates => { stats_lock.torrents_updates += value; }
            StatsEvent::TorrentsShadow => { stats_lock.torrents_shadow += value; }
            StatsEvent::Users => { stats_lock.users += value; }
            StatsEvent::UsersUpdates => { stats_lock.users_updates += value; }
            StatsEvent::UsersShadow => { stats_lock.users_shadow += value; }
            StatsEvent::TimestampSave => { stats_lock.timestamp_run_save += value; }
            StatsEvent::TimestampTimeout => { stats_lock.timestamp_run_timeout += value; }
            StatsEvent::TimestampConsole => { stats_lock.timestamp_run_console += value; }
            StatsEvent::TimestampKeysTimeout => { stats_lock.timestamp_run_keys_timeout += value; }
            StatsEvent::MaintenanceMode => { stats_lock.maintenance_mode += value; }
            StatsEvent::Seeds => { stats_lock.seeds += value; }
            StatsEvent::Peers => { stats_lock.peers += value; }
            StatsEvent::Completed => { stats_lock.completed += value; }
            StatsEvent::Whitelist => { stats_lock.whitelist += value; }
            StatsEvent::Blacklist => { stats_lock.blacklist += value; }
            StatsEvent::Key => { stats_lock.keys += value; }
            StatsEvent::Tcp4ConnectionsHandled => { stats_lock.tcp4_connections_handled += value; }
            StatsEvent::Tcp4ApiHandled => { stats_lock.tcp4_api_handled += value; }
            StatsEvent::Tcp4AnnouncesHandled => { stats_lock.tcp4_announces_handled += value; }
            StatsEvent::Tcp4ScrapesHandled => { stats_lock.tcp4_scrapes_handled += value; }
            StatsEvent::Tcp6ConnectionsHandled => { stats_lock.tcp6_connections_handled += value; }
            StatsEvent::Tcp6ApiHandled => { stats_lock.tcp6_api_handled += value; }
            StatsEvent::Tcp6AnnouncesHandled => { stats_lock.tcp6_announces_handled += value; }
            StatsEvent::Tcp6ScrapesHandled => { stats_lock.tcp6_scrapes_handled += value; }
            StatsEvent::Udp4ConnectionsHandled => { stats_lock.udp4_connections_handled += value; }
            StatsEvent::Udp4AnnouncesHandled => { stats_lock.udp4_announces_handled += value; }
            StatsEvent::Udp4ScrapesHandled => { stats_lock.udp4_scrapes_handled += value; }
            StatsEvent::Udp6ConnectionsHandled => { stats_lock.udp6_connections_handled += value; }
            StatsEvent::Udp6AnnouncesHandled => { stats_lock.udp6_announces_handled += value; }
            StatsEvent::Udp6ScrapesHandled => { stats_lock.udp6_scrapes_handled += value; }
        }
        let stats = stats_lock.clone();
        drop(stats_lock);

        stats
    }

    pub async fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let stats_arc = self.stats.clone();
        let mut stats_lock = stats_arc.write().await;
        match event {
            StatsEvent::Torrents => { stats_lock.torrents = value; }
            StatsEvent::TorrentsUpdates => { stats_lock.torrents_updates = value; }
            StatsEvent::TorrentsShadow => { stats_lock.torrents_shadow = value; }
            StatsEvent::Users => { stats_lock.users = value; }
            StatsEvent::UsersUpdates => { stats_lock.users_updates = value; }
            StatsEvent::UsersShadow => { stats_lock.users_shadow = value; }
            StatsEvent::TimestampSave => { stats_lock.timestamp_run_save = value; }
            StatsEvent::TimestampTimeout => { stats_lock.timestamp_run_timeout = value; }
            StatsEvent::TimestampConsole => { stats_lock.timestamp_run_console = value; }
            StatsEvent::TimestampKeysTimeout => { stats_lock.timestamp_run_keys_timeout = value; }
            StatsEvent::MaintenanceMode => { stats_lock.maintenance_mode = value; }
            StatsEvent::Seeds => { stats_lock.seeds = value; }
            StatsEvent::Peers => { stats_lock.peers = value; }
            StatsEvent::Completed => { stats_lock.completed = value; }
            StatsEvent::Whitelist => { stats_lock.whitelist = value; }
            StatsEvent::Blacklist => { stats_lock.blacklist = value; }
            StatsEvent::Key => { stats_lock.keys = value; }
            StatsEvent::Tcp4ConnectionsHandled => { stats_lock.tcp4_connections_handled = value; }
            StatsEvent::Tcp4ApiHandled => { stats_lock.tcp4_api_handled = value; }
            StatsEvent::Tcp4AnnouncesHandled => { stats_lock.tcp4_announces_handled = value; }
            StatsEvent::Tcp4ScrapesHandled => { stats_lock.tcp4_scrapes_handled = value; }
            StatsEvent::Tcp6ConnectionsHandled => { stats_lock.tcp6_connections_handled = value; }
            StatsEvent::Tcp6ApiHandled => { stats_lock.tcp6_api_handled = value; }
            StatsEvent::Tcp6AnnouncesHandled => { stats_lock.tcp6_announces_handled = value; }
            StatsEvent::Tcp6ScrapesHandled => { stats_lock.tcp6_scrapes_handled = value; }
            StatsEvent::Udp4ConnectionsHandled => { stats_lock.udp4_connections_handled = value; }
            StatsEvent::Udp4AnnouncesHandled => { stats_lock.udp4_announces_handled = value; }
            StatsEvent::Udp4ScrapesHandled => { stats_lock.udp4_scrapes_handled = value; }
            StatsEvent::Udp6ConnectionsHandled => { stats_lock.udp6_connections_handled = value; }
            StatsEvent::Udp6AnnouncesHandled => { stats_lock.udp6_announces_handled = value; }
            StatsEvent::Udp6ScrapesHandled => { stats_lock.udp6_scrapes_handled = value; }
        }
        let stats = stats_lock.clone();
        drop(stats_lock);

        stats
    }

    /* === Torrents === */
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

    pub async fn save_torrents(&self) -> bool
    {
        let shadow = self.get_shadow().await;
        if self.sqlx.save_torrents(shadow).await.is_ok() {
            return true;
        }
        false
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

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntryItem, persistent: bool)
    {
        let torrents_arc = self.map_torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let _ = torrents_lock.insert(info_hash, torrent_entry.clone());
        drop(torrents_lock);

        if persistent {
            self.add_update(
                info_hash,
                torrent_entry.completed,
            ).await;
        }

        self.update_stats(StatsEvent::Torrents, 1).await;
    }

    pub async fn add_torrents(&self, torrents: HashMap<InfoHash, TorrentEntryItem>, persistent: bool)
    {
        let torrents_arc = self.map_torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let mut updates = HashMap::new();
        for (info_hash, torrent_entry) in torrents.iter() {
            let _ = torrents_lock.insert(*info_hash, torrent_entry.clone());
            updates.insert(*info_hash, torrent_entry.completed);
        }
        drop(torrents_lock);

        if persistent {
            self.add_updates(
                updates
            ).await;
        }

        self.update_stats(StatsEvent::Torrents, torrents.len() as i64).await;
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let torrents_arc = self.map_torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let torrent = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent {
            None => { None }
            Some(data) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.write().await;
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

        for info_hash in hashes.iter() {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.write().await;
            let torrent = torrents_lock.get(info_hash).cloned();
            drop(torrents_lock);

            return_torrents.insert(*info_hash, match torrent {
                None => { None }
                Some(data) => {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.write().await;
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
        let torrents_arc = self.map_torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let mut torrents_return: HashMap<InfoHash, i64> = HashMap::new();
        let mut current_count: u64 = 0;
        let mut handled_count: u64 = 0;
        for (info_hash, item) in torrents_lock.iter() {
            if current_count < skip {
                current_count = current_count.add(1);
                continue;
            }
            if handled_count >= amount {
                break;
            }
            torrents_return.insert(*info_hash, item.completed);
            current_count = current_count.add(1);
            handled_count = handled_count.add(1);
        }
        drop(torrents_lock);

        torrents_return
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
            let torrents_lock = torrents_arc.write().await;
            let torrent_option = torrents_lock.get(info_hash).cloned();
            drop(torrents_lock);

            if torrent_option.is_some() {
                self.remove_torrent(*info_hash, persistent).await;
            }
        }
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
        let torrents_lock = torrents_arc.write().await;
        let torrent_input = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.write().await;
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
        let torrents_lock = torrents_arc.write().await;
        let torrent_input = torrents_lock.get(&info_hash).cloned();
        drop(torrents_lock);

        let torrent = match torrent_input {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.write().await;
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
            let torrents_lock = torrents_arc.write().await;
            let torrent = torrents_lock.get(info_hash).cloned();
            drop(torrents_lock);

            return_torrententries.insert(*info_hash, match torrent {
                None => { TorrentEntry::new() }
                Some(mut data_torrent) => {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.write().await;
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
            let peers_lock = peers_arc.write().await;
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
        let updates_lock = updates_arc.write().await;
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
        let shadow_lock = shadow_arc.write().await;
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
        let whitelist_lock = whitelist_arc.write().await;
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
        let whitelist_lock = whitelist_arc.write().await;
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
        let blacklist_lock = blacklist_arc.write().await;
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
        let blacklist_lock = blacklist_arc.write().await;
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
        let keys_lock = keys_arc.write().await;
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
        let keys_lock = keys_arc.write().await;
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
        let keys_lock = keys_arc.write().await;
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
