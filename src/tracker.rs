use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use log::info;
use scc::ebr::Arc;
use tokio::sync::RwLock;
use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
use crate::config::Configuration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::databases::DatabaseConnector;

pub enum StatsEvent {
    Torrents,
    TorrentsUpdates,
    TorrentsShadow,
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

pub struct Torrents {
    pub map_torrents: BTreeMap<InfoHash, TorrentEntryItem>,
    pub map_peers: BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>,
    pub updates: HashMap<InfoHash, i64>,
    pub shadow: HashMap<InfoHash, i64>,
    pub stats: Stats,
    pub whitelist: HashMap<InfoHash, i64>,
    pub blacklist: HashMap<InfoHash, i64>,
    pub keys: HashMap<InfoHash, i64>,
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
    pub torrents: Arc<RwLock<Torrents>>,
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        TorrentTracker {
            config: config.clone(),
            torrents: Arc::new(RwLock::new(Torrents {
                map_torrents: BTreeMap::new(),
                map_peers: BTreeMap::new(),
                updates: HashMap::new(),
                shadow: HashMap::new(),
                stats: Stats {
                    started: Utc::now().timestamp(),
                    timestamp_run_save: 0,
                    timestamp_run_timeout: 0,
                    timestamp_run_console: 0,
                    timestamp_run_keys_timeout: 0,
                    torrents: 0,
                    torrents_updates: 0,
                    torrents_shadow: 0,
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
                },
                whitelist: HashMap::new(),
                blacklist: HashMap::new(),
                keys: HashMap::new(),
            })),
            sqlx: DatabaseConnector::new(config.clone()).await,
        }
    }

    /* === Statistics === */
    pub async fn get_stats(&self) -> Stats
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let stats = torrents_lock.stats.clone();
        drop(torrents_lock);
        stats
    }

    pub async fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let mut stats = torrents_lock.stats.clone();
        match event {
            StatsEvent::Torrents => { stats.torrents += value; }
            StatsEvent::TorrentsUpdates => { stats.torrents_updates += value; }
            StatsEvent::TorrentsShadow => { stats.torrents_shadow += value; }
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
        }
        torrents_lock.stats = stats.clone();
        drop(torrents_lock);
        stats
    }

    pub async fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let mut stats = torrents_lock.stats.clone();
        match event {
            StatsEvent::Torrents => { stats.torrents = value; }
            StatsEvent::TorrentsUpdates => { stats.torrents_updates = value; }
            StatsEvent::TorrentsShadow => { stats.torrents_shadow = value; }
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
        torrents_lock.stats = stats.clone();
        drop(torrents_lock);
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
                self.add_blacklist(*info_hash).await;
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
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let _ = torrents_lock.map_torrents.insert(info_hash, torrent_entry.clone());
        drop(torrents_lock);

        if persistent {
            self.add_update(
                info_hash,
                torrent_entry.completed,
            ).await;
        }

        self.update_stats(StatsEvent::Torrents, 1).await;
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let torrent = match torrents_lock.map_torrents.get(&info_hash).cloned() {
            None => { None }
            Some(data) => {
                let peers = match torrents_lock.map_peers.get(&info_hash).cloned() {
                    None => { BTreeMap::new() }
                    Some(data) => { data }
                };
                Some(TorrentEntry {
                    peers,
                    completed: data.completed,
                    seeders: data.seeders,
                    leechers: data.leechers,
                })
            }
        };
        drop(torrents_lock);

        torrent
    }

    pub async fn get_torrents(&self, skip: u64, amount: u64) -> HashMap<InfoHash, i64>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let mut torrents_return: HashMap<InfoHash, i64> = HashMap::new();
        let mut current_count: u64 = 0;
        let mut handled_count: u64 = 0;
        for (info_hash, item) in torrents_lock.map_torrents.iter() {
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

        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let torrent_option = torrents_lock.map_torrents.get(&info_hash);
        match torrent_option {
            None => {}
            Some(data) => {
                removed_torrent = true;
                remove_seeders = data.seeders;
                remove_leechers = data.leechers;
                torrents_lock.map_torrents.remove(&info_hash);
                torrents_lock.map_peers.remove(&info_hash);
            }
        }
        drop(torrents_lock);

        if persistent {
            self.remove_update(info_hash).await;
            self.remove_shadow(info_hash).await;
        }

        if removed_torrent { self.update_stats(StatsEvent::Torrents, -1).await; }
        if remove_seeders > 0 { self.update_stats(StatsEvent::Seeds, 0 - remove_seeders).await; }
        if remove_leechers > 0 { self.update_stats(StatsEvent::Peers, 0 - remove_leechers).await; }
    }

    /* === Peers === */
    pub async fn add_peer(&self, info_hash: InfoHash, peer_id: PeerId, peer_entry: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let mut added_seeder = false;
        let mut added_leecher = false;
        let mut removed_seeder = false;
        let mut removed_leecher = false;
        let mut completed_applied = false;

        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let torrent = match torrents_lock.map_torrents.get(&info_hash).cloned() {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let mut peers = match torrents_lock.map_peers.get(&info_hash).cloned() {
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
                torrents_lock.map_torrents.insert(info_hash, data_torrent.clone());
                torrents_lock.map_peers.insert(info_hash, peers.clone());
                TorrentEntry {
                    peers,
                    completed: data_torrent.completed,
                    seeders: data_torrent.seeders,
                    leechers: data_torrent.leechers,
                }
            }
        };
        drop(torrents_lock);
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

        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let torrent = match torrents_lock.map_torrents.get(&info_hash).cloned() {
            None => { TorrentEntry::new() }
            Some(mut data_torrent) => {
                let mut peers = match torrents_lock.map_peers.get(&info_hash).cloned() {
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
                torrents_lock.map_torrents.insert(info_hash, data_torrent.clone());
                if peers.is_empty() {
                    torrents_lock.map_peers.remove(&info_hash);
                } else {
                    torrents_lock.map_peers.insert(info_hash, peers.clone());
                }
                TorrentEntry {
                    peers,
                    completed: data_torrent.completed,
                    seeders: data_torrent.seeders,
                    leechers: data_torrent.leechers,
                }
            }
        };
        drop(torrents_lock);

        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
        torrent
    }

    pub async fn clean_peers(&self, peer_timeout: Duration)
    {
        // Cleaning up peers in chunks, to prevent slow behavior.
        let mut start: usize = 0;
        let size: usize = 100000;

        loop {
            info!("[PEERS] Scanning peers {} to {}", start, (start + size));

            let torrents_arc = self.torrents.clone();
            let torrents_lock = torrents_arc.write().await;
            let mut torrent_index = vec![];
            for (info_hash, _) in torrents_lock.map_peers.iter().skip(start) {
                torrent_index.push(*info_hash);
                if torrent_index.len() == size {
                    break;
                }
            }
            drop(torrents_lock);
            drop(torrents_arc);

            for info_hash in torrent_index.iter() {
                let torrent_option = self.get_torrent(*info_hash).await.clone();
                if torrent_option.is_some() {
                    let torrent = torrent_option.unwrap().clone();
                    for (peer_id, torrent_peer) in torrent.peers.iter() {
                        if torrent_peer.updated.elapsed() > peer_timeout {
                            let _ = self.remove_peer(*info_hash, *peer_id, self.config.clone().persistence).await;
                        }
                    }
                } else {
                    continue;
                }
            }

            if torrent_index.len() != size {
                break;
            }

            start += size;
        }
    }

    /* === Updates === */
    pub async fn add_update(&self, info_hash: InfoHash, completed: i64)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.updates.insert(info_hash, completed);
        let update_count = torrents_lock.updates.len();
        drop(torrents_lock);
        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn get_update(&self) -> HashMap<InfoHash, i64>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let updates = torrents_lock.updates.clone();
        drop(torrents_lock);
        updates
    }

    pub async fn remove_update(&self, info_hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.updates.remove(&info_hash);
        let update_count = torrents_lock.updates.len();
        drop(torrents_lock);
        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn transfer_updates_to_shadow(&self)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let updates = torrents_lock.updates.clone();
        torrents_lock.updates = HashMap::new();
        drop(torrents_lock);
        for (info_hash, completed) in updates.iter() {
            self.add_shadow(*info_hash, *completed).await;
        }
        self.set_stats(StatsEvent::TorrentsUpdates, 0).await;
    }

    /* === Shadow === */
    pub async fn add_shadow(&self, info_hash: InfoHash, completed: i64)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.shadow.insert(info_hash, completed);
        let shadow_count = torrents_lock.shadow.len();
        drop(torrents_lock);
        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_shadow(&self, info_hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.shadow.remove(&info_hash);
        let shadow_count = torrents_lock.shadow.len();
        drop(torrents_lock);
        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn get_shadow(&self) -> HashMap<InfoHash, i64>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let shadow = torrents_lock.shadow.clone();
        drop(torrents_lock);
        shadow
    }

    pub async fn clear_shadow(&self)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.shadow = HashMap::new();
        drop(torrents_lock);
        self.set_stats(StatsEvent::TorrentsShadow, 0).await;
    }

    /* === Whitelist === */
    pub async fn add_whitelist(&self, info_hash: InfoHash, on_load: bool)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        if on_load {
            torrents_lock.whitelist.insert(info_hash, 1i64);
        } else {
            torrents_lock.whitelist.insert(info_hash, 2i64);
        }
        drop(torrents_lock);
        self.update_stats(StatsEvent::Whitelist, 1).await;
    }

    pub async fn get_whitelist(&self) -> HashMap<InfoHash, i64>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let mut return_list = HashMap::new();
        for (info_hash, value) in torrents_lock.whitelist.iter() {
            return_list.insert(*info_hash, *value);
        }
        drop(torrents_lock);
        return_list
    }

    pub async fn remove_flag_whitelist(&self, info_hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        if torrents_lock.whitelist.get(&info_hash).is_some() {
            torrents_lock.whitelist.insert(info_hash, 0i64);
        }
        let mut whitelist_count = 0i64;
        for (_, value) in torrents_lock.whitelist.iter() {
            if value == &1i64 {
                whitelist_count += 1;
            }
        }
        drop(torrents_lock);
        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn remove_whitelist(&self, info_hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.whitelist.remove(&info_hash);
        let mut whitelist_count = 0i64;
        for (_, value) in torrents_lock.whitelist.iter() {
            if value == &1 {
                whitelist_count += 1;
            }
        }
        drop(torrents_lock);
        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let whitelist = torrents_lock.whitelist.get(&info_hash).cloned();
        drop(torrents_lock);
        if whitelist.is_some() {
            return true;
        }
        false
    }

    pub async fn clear_whitelist(&self)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.whitelist = HashMap::new();
        drop(torrents_lock);
        self.set_stats(StatsEvent::Whitelist, 0).await;
    }

    /* === Blacklist === */
    pub async fn add_blacklist(&self, info_hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.blacklist.insert(info_hash, 0i64);
        drop(torrents_lock);
        self.update_stats(StatsEvent::Blacklist, 1).await;
    }

    pub async fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let mut return_list = vec![];
        for (info_hash, _) in torrents_lock.blacklist.iter() {
            return_list.push(*info_hash);
        }
        drop(torrents_lock);
        return_list
    }

    pub async fn remove_blacklist(&self, info_hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.blacklist.remove(&info_hash);
        let blacklist_count = torrents_lock.blacklist.len();
        drop(torrents_lock);
        self.set_stats(StatsEvent::Blacklist, blacklist_count as i64).await;
    }

    pub async fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let blacklist = torrents_lock.blacklist.get(&info_hash).cloned();
        drop(torrents_lock);
        if blacklist.is_some() {
            return true;
        }
        false
    }

    pub async fn clear_blacklist(&self)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.whitelist = HashMap::new();
        drop(torrents_lock);
        self.set_stats(StatsEvent::Blacklist, 0).await;
    }

    /* === Keys === */
    pub async fn add_key(&self, hash: InfoHash, timeout: i64)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_unix = timestamp.as_secs() as i64 + timeout;
        torrents_lock.keys.insert(hash, timeout_unix);
        drop(torrents_lock);
        self.update_stats(StatsEvent::Key, 1).await;
    }

    pub async fn add_key_raw(&self, hash: InfoHash, timeout: i64)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        let time = SystemTime::from(Utc.timestamp_opt(timeout, 0).unwrap());
        match time.duration_since(SystemTime::now()) {
            Ok(_) => {
                torrents_lock.keys.insert(hash, timeout);
            }
            Err(_) => {
                drop(torrents_lock);
                return;
            }
        }
        drop(torrents_lock);
        self.update_stats(StatsEvent::Key, 1).await;
    }

    pub async fn get_keys(&self) -> Vec<(InfoHash, i64)>
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let mut return_list = vec![];
        for (hash, timeout) in torrents_lock.keys.iter() {
            return_list.push((*hash, *timeout));
        }
        drop(torrents_lock);
        return_list
    }

    pub async fn remove_key(&self, hash: InfoHash)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.keys.remove(&hash);
        let key_count = torrents_lock.keys.len();
        drop(torrents_lock);
        self.set_stats(StatsEvent::Key, key_count as i64).await;
    }

    pub async fn check_key(&self, hash: InfoHash) -> bool
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;
        let key = torrents_lock.keys.get(&hash).cloned();
        drop(torrents_lock);
        if key.is_some() {
            return true;
        }
        false
    }

    pub async fn clear_keys(&self)
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_lock = torrents_arc.write().await;
        torrents_lock.keys = HashMap::new();
        drop(torrents_lock);
        self.set_stats(StatsEvent::Key, 0).await;
    }

    pub async fn clean_keys(&self)
    {
        let torrents_arc = self.torrents.clone();
        let torrents_lock = torrents_arc.write().await;

        let mut keys_index = vec![];
        for (hash, timeout) in torrents_lock.keys.iter() {
            keys_index.push((*hash, *timeout));
        }
        drop(torrents_lock);

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
