//! Main tracker instance definition.

use crate::cache::structs::cache_connector::CacheConnector;
use crate::config::structs::configuration::Configuration;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::ssl::certificate_store::CertificateStore;
use crate::stats::structs::stats_atomics::StatsAtomics;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use crate::tracker::types::keys_updates::KeysUpdates;
use crate::tracker::types::torrents_updates::TorrentsUpdates;
use crate::tracker::types::users_updates::UsersUpdates;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

/// The main BitTorrent tracker instance.
///
/// `TorrentTracker` is the central struct that manages all tracker state and operations.
/// It holds references to the database, cache, certificates, torrents, users, and statistics.
///
/// # Architecture
///
/// The tracker uses several concurrent data structures:
/// - **Torrents**: Stored in a sharded structure ([`TorrentSharding`]) with 256 shards
/// - **Updates**: Pending database changes are batched in thread-safe maps
/// - **Statistics**: Atomic counters for real-time metrics
///
/// # Thread Safety
///
/// All fields are designed for concurrent access:
/// - `Arc` for shared ownership across threads
/// - `RwLock` from `parking_lot` for efficient read-heavy workloads
/// - Atomic types for statistics counters
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use torrust_actix::config::structs::configuration::Configuration;
/// use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
///
/// let config = Arc::new(Configuration::default());
/// let tracker = TorrentTracker::new(config, false).await;
///
/// // Access torrent count
/// let count = tracker.torrents_sharding.get_torrents_amount();
/// ```
#[derive(Debug)]
pub struct TorrentTracker {
    /// Shared configuration for the tracker.
    pub config: Arc<Configuration>,

    /// Database connector for persistent storage.
    pub sqlx: DatabaseConnector,

    /// Optional cache connector (Redis or Memcache).
    pub cache: Option<CacheConnector>,

    /// SSL/TLS certificate store for HTTPS endpoints.
    pub certificate_store: Arc<CertificateStore>,

    /// Sharded storage for torrent entries (256 shards).
    pub torrents_sharding: Arc<TorrentSharding>,

    /// Pending torrent updates to be flushed to database.
    pub torrents_updates: TorrentsUpdates,

    /// Set of whitelisted info hashes (when whitelist is enabled).
    pub torrents_whitelist: Arc<RwLock<HashSet<InfoHash>>>,

    /// Pending whitelist updates to be flushed to database.
    pub torrents_whitelist_updates: Arc<RwLock<HashMap<u128, (InfoHash, UpdatesAction)>>>,

    /// Set of blacklisted info hashes (when blacklist is enabled).
    pub torrents_blacklist: Arc<RwLock<HashSet<InfoHash>>>,

    /// Pending blacklist updates to be flushed to database.
    pub torrents_blacklist_updates: Arc<RwLock<HashMap<u128, (InfoHash, UpdatesAction)>>>,

    /// API keys with their expiration timestamps.
    pub keys: Arc<RwLock<BTreeMap<InfoHash, i64>>>,

    /// Pending API key updates to be flushed to database.
    pub keys_updates: KeysUpdates,

    /// User accounts with their statistics.
    pub users: Arc<RwLock<BTreeMap<UserId, UserEntryItem>>>,

    /// Pending user updates to be flushed to database.
    pub users_updates: UsersUpdates,

    /// Atomic statistics counters for real-time metrics.
    pub stats: Arc<StatsAtomics>,
}