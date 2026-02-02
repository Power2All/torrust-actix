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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_display() {
        let error = CacheError::ConnectionError("failed to connect".to_string());
        assert_eq!(format!("{}", error), "Connection error: failed to connect");
    }

    #[test]
    fn test_operation_error_display() {
        let error = CacheError::OperationError("operation failed".to_string());
        assert_eq!(format!("{}", error), "Operation error: operation failed");
    }

    #[test]
    fn test_serialization_error_display() {
        let error = CacheError::SerializationError("invalid data".to_string());
        assert_eq!(format!("{}", error), "Serialization error: invalid data");
    }

    #[test]
    fn test_key_not_found_display() {
        let error = CacheError::KeyNotFound("test_key".to_string());
        assert_eq!(format!("{}", error), "Key not found: test_key");
    }

    #[test]
    fn test_error_debug() {
        let error = CacheError::ConnectionError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ConnectionError"));
        assert!(debug_str.contains("test"));
    }
}