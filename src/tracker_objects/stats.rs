use async_std::future::timeout;
use log::error;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::tracker::TorrentTracker;

#[derive(Serialize, Deserialize, Clone, Copy)]
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

#[derive(Serialize, Deserialize, Clone, Copy)]
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

impl TorrentTracker {
    pub async fn get_stats(&self) -> Result<Stats, ()>
    {
        match timeout(Duration::from_secs(30), async move {
            let stats_arc = self.stats.clone();
            let stats_lock = stats_arc.lock().await;
            let stats = *stats_lock;
            drop(stats_lock);
            stats
        }).await {
            Ok(data) => { Ok(data) }
            Err(_) => { error!("[GET_STATS] Read Lock (stats) request timed out!"); Err(()) }
        }
    }

    pub async fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let mut count = 1;
        loop {
            match timeout(Duration::from_secs(30), async move {
                let stats_arc = self.stats.clone();
                let mut stats_lock = stats_arc.lock().await;
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
                let stats = *stats_lock;
                drop(stats_lock);
                stats
            }).await {
                Ok(stats) => { return stats; }
                Err(_) => {
                    if count == 5 { panic!("[UPDATE_STATS] Write Lock (stats) request timed out, giving up..."); }
                    error!("[UPDATE_STATS] Write Lock (stats) request timed out, retrying {} time(s)...", count);
                    count += 1;
                }
            }
        }
    }

    pub async fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let mut count = 1;
        loop {
            match timeout(Duration::from_secs(30), async move {
                let stats_arc = self.stats.clone();
                let mut stats_lock = stats_arc.lock().await;
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
                let stats = *stats_lock;
                drop(stats_lock);
                stats
            }).await {
                Ok(stats) => { return stats; }
                Err(_) => {
                    if count == 5 { panic!("[SET_STATS] Write Lock (stats) request timed out, giving up..."); }
                    error!("[SET_STATS] Write Lock (stats) request timed out, retrying...");
                    count += 1;
                }
            }
        }
    }
}