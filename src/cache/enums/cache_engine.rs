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