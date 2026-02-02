use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CacheEngine {
    redis,
    memcache,
}

impl fmt::Display for CacheEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheEngine::redis => write!(f, "redis"),
            CacheEngine::memcache => write!(f, "memcache"),
        }
    }
}

impl CacheEngine {
    pub fn url_scheme(&self) -> &'static str {
        match self {
            CacheEngine::redis => "redis://",
            CacheEngine::memcache => "memcache://",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_engine_display() {
        assert_eq!(format!("{}", CacheEngine::redis), "redis");
        assert_eq!(format!("{}", CacheEngine::memcache), "memcache");
    }

    #[test]
    fn test_cache_engine_url_scheme() {
        assert_eq!(CacheEngine::redis.url_scheme(), "redis://");
        assert_eq!(CacheEngine::memcache.url_scheme(), "memcache://");
    }

    #[test]
    fn test_cache_engine_serialization() {
        let redis_engine = CacheEngine::redis;
        let serialized = serde_json::to_string(&redis_engine).unwrap();
        assert_eq!(serialized, "\"redis\"");
        let memcache_engine = CacheEngine::memcache;
        let serialized = serde_json::to_string(&memcache_engine).unwrap();
        assert_eq!(serialized, "\"memcache\"");
    }

    #[test]
    fn test_cache_engine_deserialization() {
        let redis_engine: CacheEngine = serde_json::from_str("\"redis\"").unwrap();
        assert_eq!(redis_engine, CacheEngine::redis);
        let memcache_engine: CacheEngine = serde_json::from_str("\"memcache\"").unwrap();
        assert_eq!(memcache_engine, CacheEngine::memcache);
    }

    #[test]
    fn test_cache_engine_ordering() {
        assert!(CacheEngine::redis < CacheEngine::memcache);
    }

    #[test]
    fn test_cache_engine_clone() {
        let redis_engine = CacheEngine::redis;
        let cloned = redis_engine.clone();
        assert_eq!(redis_engine, cloned);
    }
}