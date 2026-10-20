use crate::cache::structs::cache_connector::CacheConnector;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::ssl::structs::certificate_store::CertificateStore;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::types::ahash_map::AHashMap;
use log::{
    info,
    warn
};
use parking_lot::RwLock;
use std::collections::{
    BTreeMap,
    HashMap,
    HashSet
};
use std::sync::atomic::{
    AtomicBool,
    AtomicI64
};
use std::sync::Arc;

impl TorrentTracker {
    /// Creates the central tracker state: connects to the configured database (optionally
    /// creating its schema) and cache backend, and initialises empty swarm/user/key tables
    /// and statistics.
    pub async fn new(config: Arc<Configuration>, create_database: bool) -> TorrentTracker
    {
        let tracker_config = &config.tracker_config;
        let cache = if let Some(ref cache_config) = config.cache {
            if cache_config.enabled {
                match CacheConnector::new(cache_config).await {
                    Ok(connector) => {
                        info!("[Cache] Successfully connected to {} cache", match cache_config.engine {
                            crate::cache::enums::cache_engine::CacheEngine::redis => "Redis",
                            crate::cache::enums::cache_engine::CacheEngine::memcache => "Memcache",
                        });
                        Some(connector)
                    }
                    Err(e) => {
                        warn!("[Cache] Failed to connect to cache: {e}. Continuing without cache.");
                        None
                    }
                }
            } else {
                info!("[Cache] Cache is disabled in configuration");
                None
            }
        } else {
            None
        };
        TorrentTracker {
            config: config.clone(),
            cache,
            certificate_store: Arc::new(CertificateStore::new()),
            torrents_sharding: Arc::new(Default::default()),
            torrents_updates: Arc::new(Default::default()),
            torrents_whitelist: Arc::new(RwLock::new(HashSet::new())),
            torrents_whitelist_updates: Arc::new(RwLock::new(HashMap::new())),
            torrents_blacklist: Arc::new(RwLock::new(HashSet::new())),
            torrents_blacklist_updates: Arc::new(RwLock::new(HashMap::new())),
            keys: Arc::new(RwLock::new(BTreeMap::new())),
            keys_updates: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(StatsAtomics {
                started: AtomicI64::new(crate::common::common::current_time().cast_signed()),
                whitelist_enabled: AtomicBool::new(tracker_config.whitelist_enabled),
                blacklist_enabled: AtomicBool::new(tracker_config.blacklist_enabled),
                keys_enabled: AtomicBool::new(tracker_config.keys_enabled),
                ..Default::default()
            }),
            users: Arc::new(RwLock::new(BTreeMap::new())),
            users_key_index: Arc::new(RwLock::new(AHashMap::default())),
            users_updates: Arc::new(RwLock::new(HashMap::new())),
            sqlx: DatabaseConnector::new(config.clone(), create_database).await,
        }
    }
}