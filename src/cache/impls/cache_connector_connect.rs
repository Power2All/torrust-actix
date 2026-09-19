use crate::cache::enums::cache_engine::CacheEngine;
use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector::CacheConnector;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::config::structs::cache_config::CacheConfig;
use log::info;

impl CacheConnector {
    /// Connects to the configured cache engine (Redis or Memcache).
    ///
    /// # Errors
    ///
    /// Returns a [`CacheError`] when the backend cannot be initialised.
    pub async fn new(config: &CacheConfig) -> Result<CacheConnector, CacheError> {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("cache_init", "cache");
        let connection_url = format!("{}{}", config.engine.url_scheme(), config.address);
        let result: Result<CacheConnector, CacheError> = match config.engine {
            CacheEngine::redis => {
                let redis_connector = CacheConnectorRedis::connect(&connection_url, &config.prefix, config.split_peers).await?;
                info!("[Cache] Connected to Redis at {} (split_peers={})", config.address, config.split_peers);
                Ok(CacheConnector::Redis(redis_connector))
            }
            CacheEngine::memcache => {
                let memcache_connector = CacheConnectorMemcache::connect(&connection_url, &config.prefix, config.split_peers)?;
                info!("[Cache] Connected to Memcache at {} (split_peers={})", config.address, config.split_peers);
                Ok(CacheConnector::Memcache(memcache_connector))
            }
        };
        if let Some(txn) = transaction {
            match &result {
                Ok(_) => txn.set_tag("result", "success"),
                Err(e) => txn.set_tag("result", format!("error: {e:?}")),
            }
            txn.set_tag("engine", format!("{:?}", config.engine));
            txn.set_tag("address", config.address.clone());
            txn.finish();
        }
        result
    }

}