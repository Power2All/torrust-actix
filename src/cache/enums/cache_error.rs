use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Memcache error: {0}")]
    MemcacheError(#[from] memcache::MemcacheError),
}