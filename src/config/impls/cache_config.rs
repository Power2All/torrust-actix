use crate::cache::enums::cache_engine::CacheEngine;
use crate::config::structs::cache_config::CacheConfig;

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: CacheEngine::redis,
            address: "127.0.0.1:6379".to_string(),
            prefix: "tracker:".to_string(),
            ttl: 300,
            split_peers: false,
        }
    }
}