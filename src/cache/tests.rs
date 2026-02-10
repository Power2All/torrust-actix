#[cfg(test)]
mod cache_tests {
    mod cache_engine_tests {
        use crate::cache::enums::cache_engine::CacheEngine;

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

    mod error_tests {
        use crate::cache::enums::cache_error::CacheError;

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
}