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
    pub split_peers: bool,
}