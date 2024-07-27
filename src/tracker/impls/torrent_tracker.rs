use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64};
use chrono::Utc;
use crossbeam_skiplist::SkipMap;
use parking_lot::RwLock;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        TorrentTracker {
            config: config.clone(),
            torrents_sharding: Arc::new(Default::default()),
            torrents_map: Arc::new(RwLock::new(BTreeMap::new())),
            torrents_updates: Arc::new(SkipMap::new()),
            torrents_shadow: Arc::new(SkipMap::new()),
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
                test_counter: AtomicI64::new(0),
                test_counter_udp: AtomicI64::new(0)
            }),
            torrents_whitelist: Arc::new(SkipMap::new()),
            torrents_blacklist: Arc::new(SkipMap::new()),
            keys: Arc::new(SkipMap::new()),
            users: Arc::new(SkipMap::new()),
            users_updates: Arc::new(SkipMap::new()),
            users_shadow: Arc::new(SkipMap::new()),
            sqlx: DatabaseConnector::new(config.clone()).await,
        }
    }
}
