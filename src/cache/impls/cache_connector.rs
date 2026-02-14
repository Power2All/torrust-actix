use crate::cache::enums::cache_engine::CacheEngine;
use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector::CacheConnector;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::config::structs::cache_config::CacheConfig;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;
use log::info;

impl CacheConnector {
    pub async fn new(config: &CacheConfig) -> Result<CacheConnector, CacheError> {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("cache_init", "cache");
        let connection_url = format!("{}{}", config.engine.url_scheme(), config.address);
        let result: Result<CacheConnector, CacheError> = match config.engine {
            CacheEngine::redis => {
                let redis_connector = CacheConnectorRedis::connect(&connection_url, &config.prefix).await?;
                info!("[Cache] Connected to Redis at {}", config.address);
                Ok(CacheConnector {
                    redis: Some(redis_connector),
                    memcache: None,
                    engine: Some(CacheEngine::redis),
                })
            }
            CacheEngine::memcache => {
                let memcache_connector = CacheConnectorMemcache::connect(&connection_url, &config.prefix)?;
                info!("[Cache] Connected to Memcache at {}", config.address);
                Ok(CacheConnector {
                    redis: None,
                    memcache: Some(memcache_connector),
                    engine: Some(CacheEngine::memcache),
                })
            }
        };
        if let Some(txn) = transaction {
            match &result {
                Ok(_) => txn.set_tag("result", "success"),
                Err(e) => txn.set_tag("result", format!("error: {:?}", e)),
            }
            txn.set_tag("engine", format!("{:?}", config.engine));
            txn.set_tag("address", config.address.clone());
            txn.finish();
        }
        result
    }

    pub fn backend(&self) -> Option<&dyn CacheBackend> {
        match self.engine.as_ref()? {
            CacheEngine::redis => self.redis.as_ref().map(|r| r as &dyn CacheBackend),
            CacheEngine::memcache => self.memcache.as_ref().map(|m| m as &dyn CacheBackend),
        }
    }
}

#[async_trait]
impl CacheBackend for CacheConnector {
    async fn ping(&self) -> Result<(), CacheError> {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("cache_ping", "cache");
        let result: Result<(), CacheError> = match self.engine.as_ref() {
            Some(CacheEngine::redis) => {
                if let Some(ref redis) = self.redis {
                    redis.ping().await
                } else {
                    Err(CacheError::ConnectionError("Redis not connected".to_string()))
                }
            }
            Some(CacheEngine::memcache) => {
                if let Some(ref memcache) = self.memcache {
                    memcache.ping().await
                } else {
                    Err(CacheError::ConnectionError("Memcache not connected".to_string()))
                }
            }
            None => Err(CacheError::ConnectionError("No cache engine configured".to_string())),
        };
        if let Some(txn) = transaction {
            match &result {
                Ok(_) => txn.set_tag("result", "success"),
                Err(e) => txn.set_tag("result", format!("error: {:?}", e)),
            }
            if let Some(engine) = &self.engine {
                txn.set_tag("engine", format!("{:?}", engine));
            }
            txn.finish();
        }
        result
    }

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        seeds: u64,
        peers: u64,
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        match self.engine.as_ref() {
            Some(CacheEngine::redis) => {
                if let Some(ref redis) = self.redis {
                    redis.set_torrent_peers(info_hash, seeds, peers, ttl).await
                } else {
                    Err(CacheError::ConnectionError("Redis not connected".to_string()))
                }
            }
            Some(CacheEngine::memcache) => {
                if let Some(ref memcache) = self.memcache {
                    memcache.set_torrent_peers(info_hash, seeds, peers, ttl).await
                } else {
                    Err(CacheError::ConnectionError("Memcache not connected".to_string()))
                }
            }
            None => Err(CacheError::ConnectionError("No cache engine configured".to_string())),
        }
    }

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<(u64, u64)>, CacheError> {
        match self.engine.as_ref() {
            Some(CacheEngine::redis) => {
                if let Some(ref redis) = self.redis {
                    redis.get_torrent_peers(info_hash).await
                } else {
                    Err(CacheError::ConnectionError("Redis not connected".to_string()))
                }
            }
            Some(CacheEngine::memcache) => {
                if let Some(ref memcache) = self.memcache {
                    memcache.get_torrent_peers(info_hash).await
                } else {
                    Err(CacheError::ConnectionError("Memcache not connected".to_string()))
                }
            }
            None => Err(CacheError::ConnectionError("No cache engine configured".to_string())),
        }
    }

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError> {
        match self.engine.as_ref() {
            Some(CacheEngine::redis) => {
                if let Some(ref redis) = self.redis {
                    redis.delete_torrent(info_hash).await
                } else {
                    Err(CacheError::ConnectionError("Redis not connected".to_string()))
                }
            }
            Some(CacheEngine::memcache) => {
                if let Some(ref memcache) = self.memcache {
                    memcache.delete_torrent(info_hash).await
                } else {
                    Err(CacheError::ConnectionError("Memcache not connected".to_string()))
                }
            }
            None => Err(CacheError::ConnectionError("No cache engine configured".to_string())),
        }
    }

    async fn set_torrent_peers_batch(
        &self,
        data: &[(InfoHash, u64, u64)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        match self.engine.as_ref() {
            Some(CacheEngine::redis) => {
                if let Some(ref redis) = self.redis {
                    redis.set_torrent_peers_batch(data, ttl).await
                } else {
                    Err(CacheError::ConnectionError("Redis not connected".to_string()))
                }
            }
            Some(CacheEngine::memcache) => {
                if let Some(ref memcache) = self.memcache {
                    memcache.set_torrent_peers_batch(data, ttl).await
                } else {
                    Err(CacheError::ConnectionError("Memcache not connected".to_string()))
                }
            }
            None => Err(CacheError::ConnectionError("No cache engine configured".to_string())),
        }
    }
}