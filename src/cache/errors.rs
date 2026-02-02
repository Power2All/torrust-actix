use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Operation error: {0}")]
    OperationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Memcache error: {0}")]
    MemcacheError(#[from] memcache::MemcacheError),
}