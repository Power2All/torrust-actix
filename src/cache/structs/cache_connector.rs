use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;

/// A connection to whichever cache engine is configured.
///
/// An enum rather than one `Option` per engine plus an `Option<CacheEngine>` discriminant:
/// [`CacheConnector::new`] is the only constructor and always fills exactly one engine, so the
/// mismatched combinations that shape allowed — engine set with no connector, two connectors at
/// once, no engine at all — were unreachable states every method still had to answer for.
#[derive(Debug, Clone)]
pub enum CacheConnector {
    Redis(CacheConnectorRedis),
    Memcache(CacheConnectorMemcache),
}
