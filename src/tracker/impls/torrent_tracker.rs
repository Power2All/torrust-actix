use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64};
use chrono::Utc;
use parking_lot::RwLock;
use uuid::Uuid;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn new(config: Arc<Configuration>, create_database: bool) -> TorrentTracker
    {
        TorrentTracker {
            server_id: String::from(""),
            config: config.clone(),
            torrents_sharding: Arc::new(TorrentSharding::new()),
            torrents_updates: Arc::new(RwLock::new(HashMap::new())),
            torrents_whitelist: Arc::new(RwLock::new(Vec::new())),
            torrents_whitelist_updates: Arc::new(RwLock::new(HashMap::new())),
            torrents_blacklist: Arc::new(RwLock::new(Vec::new())),
            torrents_blacklist_updates: Arc::new(RwLock::new(HashMap::new())),
            keys: Arc::new(RwLock::new(BTreeMap::new())),
            keys_updates: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(StatsAtomics {
                started: AtomicI64::new(Utc::now().timestamp()),
                timestamp_run_save: AtomicI64::new(0),
                timestamp_run_timeout: AtomicI64::new(0),
                timestamp_run_console: AtomicI64::new(0),
                timestamp_run_keys_timeout: AtomicI64::new(0),
                torrents: AtomicI64::new(0),
                torrents_updates: AtomicI64::new(0),
                users: AtomicI64::new(0),
                users_updates: AtomicI64::new(0),
                seeds: AtomicI64::new(0),
                peers: AtomicI64::new(0),
                completed: AtomicI64::new(0),
                whitelist_enabled: AtomicBool::new(config.tracker_config.clone().whitelist_enabled),
                whitelist: AtomicI64::new(0),
                whitelist_updates: AtomicI64::new(0),
                blacklist_enabled: AtomicBool::new(config.tracker_config.clone().blacklist_enabled),
                blacklist: AtomicI64::new(0),
                blacklist_updates: AtomicI64::new(0),
                keys_enabled: AtomicBool::new(config.tracker_config.clone().keys_enabled),
                keys: AtomicI64::new(0),
                keys_updates: AtomicI64::new(0),
                tcp4_connections_handled: AtomicI64::new(0),
                tcp4_api_handled: AtomicI64::new(0),
                tcp4_announces_handled: AtomicI64::new(0),
                tcp4_scrapes_handled: AtomicI64::new(0),
                tcp4_not_found: AtomicI64::new(0),
                tcp4_failure: AtomicI64::new(0),
                tcp6_connections_handled: AtomicI64::new(0),
                tcp6_api_handled: AtomicI64::new(0),
                tcp6_announces_handled: AtomicI64::new(0),
                tcp6_scrapes_handled: AtomicI64::new(0),
                tcp6_not_found: AtomicI64::new(0),
                tcp6_failure: AtomicI64::new(0),
                udp4_invalid_request: AtomicI64::new(0),
                udp4_bad_request: AtomicI64::new(0),
                udp4_connections_handled: AtomicI64::new(0),
                udp4_announces_handled: AtomicI64::new(0),
                udp4_scrapes_handled: AtomicI64::new(0),
                udp6_invalid_request: AtomicI64::new(0),
                udp6_bad_request: AtomicI64::new(0),
                udp6_connections_handled: AtomicI64::new(0),
                udp6_announces_handled: AtomicI64::new(0),
                udp6_scrapes_handled: AtomicI64::new(0),
            }),
            users: Arc::new(RwLock::new(BTreeMap::new())),
            users_updates: Arc::new(RwLock::new(HashMap::new())),
            sqlx: DatabaseConnector::new(config.clone(), create_database).await,
        }
    }
}