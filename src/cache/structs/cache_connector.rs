use crate::cache::enums::cache_engine::CacheEngine;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;

#[derive(Debug, Clone)]
pub struct CacheConnector {
    pub(crate) redis: Option<CacheConnectorRedis>,
    pub(crate) memcache: Option<CacheConnectorMemcache>,
    pub(crate) engine: Option<CacheEngine>,
}