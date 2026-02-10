use crate::cache::enums::cache_engine::CacheEngine;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub engine: CacheEngine,
    pub address: String,
    pub prefix: String,
    pub ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: CacheEngine::redis,
            address: "127.0.0.1:6379".to_string(),
            prefix: "tracker:".to_string(),
            ttl: 300,
        }
    }
}