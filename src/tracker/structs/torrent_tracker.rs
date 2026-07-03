use crate::cache::structs::cache_connector::CacheConnector;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::ssl::structs::certificate_store::CertificateStore;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use crate::tracker::types::ahash_map::AHashMap;
use crate::tracker::types::keys_updates::KeysUpdates;
use crate::tracker::types::torrents_updates::TorrentsUpdates;
use crate::tracker::types::users_updates::UsersUpdates;
use parking_lot::RwLock;
use std::collections::{
    BTreeMap,
    HashMap,
    HashSet
};
use std::sync::Arc;

#[derive(Debug)]
pub struct TorrentTracker {
    /// Shared application configuration.
    pub config: Arc<Configuration>,
    /// Async database connection (SQLite 3 / MySQL / PostgreSQL).
    pub sqlx: DatabaseConnector,
    /// Optional Redis or Memcache connector for peer-data caching.
    pub cache: Option<CacheConnector>,
    /// Hot-reloadable TLS certificate store used by HTTP and API servers.
    pub certificate_store: Arc<CertificateStore>,
    /// Sharded in-memory torrent map (256 shards).
    pub torrents_sharding: Arc<TorrentSharding>,
    /// Pending torrent stat changes awaiting the next database flush.
    pub torrents_updates: TorrentsUpdates,
    /// Set of info-hashes allowed when whitelist mode is active.
    pub torrents_whitelist: Arc<RwLock<HashSet<InfoHash>>>,
    /// Pending whitelist additions/removals awaiting the next database flush.
    pub torrents_whitelist_updates: Arc<RwLock<HashMap<u128, (InfoHash, UpdatesAction)>>>,
    /// Set of info-hashes blocked when blacklist mode is active.
    pub torrents_blacklist: Arc<RwLock<HashSet<InfoHash>>>,
    /// Pending blacklist additions/removals awaiting the next database flush.
    pub torrents_blacklist_updates: Arc<RwLock<HashMap<u128, (InfoHash, UpdatesAction)>>>,
    /// Active announce keys mapped to their expiry timestamp (`0` = permanent).
    pub keys: Arc<RwLock<BTreeMap<InfoHash, i64>>>,
    /// Pending key additions/removals awaiting the next database flush.
    pub keys_updates: KeysUpdates,
    /// Registered users indexed by their [`UserId`].
    pub users: Arc<RwLock<BTreeMap<UserId, UserEntryItem>>>,
    /// Secondary index mapping a user's announce key to its [`UserId`] map
    /// key.  Kept in sync by `add_user` / `remove_user` / `clear_users` so
    /// `check_user_key` is an O(1) lookup instead of a full user scan.
    pub users_key_index: Arc<RwLock<AHashMap<UserId, UserId>>>,
    /// Pending user stat changes awaiting the next database flush.
    pub users_updates: UsersUpdates,
    /// Atomic statistics counters (connections, announces, scrapes, …).
    pub stats: Arc<StatsAtomics>
}