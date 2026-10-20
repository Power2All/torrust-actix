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

    /// The URL to connect to for a `cache.address`: a whole URL (`redis://:secret@redis:6379/2`) is
    /// used as given, anything else (`host:port`, `user:pass@host:port`) gets this engine's scheme.
    pub fn connection_url(&self, address: &str) -> String {
        if address.contains("://") {
            address.to_string()
        } else {
            format!("{}{}", self.url_scheme(), address)
        }
    }
}