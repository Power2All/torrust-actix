use chrono::Utc;
use crossbeam_skiplist::SkipMap;
use async_std::sync::Arc;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicI64};

use crate::common::{InfoHash, PeerId, TorrentPeer, UserId};
use crate::config::Configuration;
use crate::databases::DatabaseConnector;
use crate::tracker_objects::stats::StatsAtomics;
use crate::tracker_objects::torrents::TorrentEntryItem;
use crate::tracker_objects::users::UserEntryItem;

pub struct TorrentTracker {
    pub config: Arc<Configuration>,
    pub torrents: Arc<SkipMap<InfoHash, TorrentEntryItem>>,
    pub peers: Arc<SkipMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>>,
    pub torrents_updates: Arc<SkipMap<InfoHash, i64>>,
    pub torrents_shadow: Arc<SkipMap<InfoHash, i64>>,
    pub stats: Arc<StatsAtomics>,
    pub torrents_whitelist: Arc<SkipMap<InfoHash, i64>>,
    pub torrents_blacklist: Arc<SkipMap<InfoHash, i64>>,
    pub keys: Arc<SkipMap<InfoHash, i64>>,
    pub users: Arc<SkipMap<UserId, UserEntryItem>>,
    pub users_updates: Arc<SkipMap<UserId, UserEntryItem>>,
    pub users_shadow: Arc<SkipMap<UserId, UserEntryItem>>,
    pub sqlx: DatabaseConnector,
}

impl TorrentTracker {
    pub async fn new(config: Arc<Configuration>) -> TorrentTracker
    {
        TorrentTracker {
            config: config.clone(),
            torrents: Arc::new(SkipMap::new()),
            peers: Arc::new(SkipMap::new()),
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
                tcp6_connections_handled: AtomicI64::new(0),
                tcp6_api_handled: AtomicI64::new(0),
                tcp6_announces_handled: AtomicI64::new(0),
                tcp6_scrapes_handled: AtomicI64::new(0),
                udp4_connections_handled: AtomicI64::new(0),
                udp4_announces_handled: AtomicI64::new(0),
                udp4_scrapes_handled: AtomicI64::new(0),
                udp6_connections_handled: AtomicI64::new(0),
                udp6_announces_handled: AtomicI64::new(0),
                udp6_scrapes_handled: AtomicI64::new(0),
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