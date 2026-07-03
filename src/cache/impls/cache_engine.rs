use crate::cache::enums::cache_engine::CacheEngine;
use std::fmt;

impl fmt::Display for CacheEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheEngine::redis => write!(f, "redis"),
            CacheEngine::memcache => write!(f, "memcache"),
        }
    }
}

impl CacheEngine {
    /// Returns the URL scheme prefix used to connect to this cache engine
    /// (`redis://` or `memcache://`).
    pub fn url_scheme(&self) -> &'static str {
        match self {
            CacheEngine::redis => "redis://",
            CacheEngine::memcache => "memcache://",
        }
    }
}