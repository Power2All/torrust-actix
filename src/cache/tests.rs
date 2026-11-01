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
        fn test_error_debug() {
            let error = CacheError::ConnectionError("test".to_string());
            let debug_str = format!("{:?}", error);
            assert!(debug_str.contains("ConnectionError"));
            assert!(debug_str.contains("test"));
        }
    }

    mod user_tests {
        use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
        use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
        use crate::cache::structs::user_stat_counts::UserStatCounts;
        use crate::tracker::structs::info_hash::InfoHash;
        use crate::tracker::structs::user_entry_item::UserEntryItem;
        use crate::tracker::structs::user_id::UserId;
        use std::collections::BTreeMap;

        fn user(user_id: Option<u64>, user_uuid: Option<&str>) -> UserEntryItem {
            UserEntryItem {
                key: UserId([0x12; 20]),
                user_id,
                user_uuid: user_uuid.map(str::to_string),
                uploaded: 5 << 30,
                downloaded: 300,
                completed: 2,
                updated: 1_700_000_000,
                active: 1,
                torrents_active: BTreeMap::from([(InfoHash([1; 20]), 1), (InfoHash([2; 20]), 2)]),
            }
        }

        #[tokio::test]
        async fn test_redis_user_key() {
            // Connecting opens no socket, so no server is needed.
            let redis = CacheConnectorRedis::connect("redis://127.0.0.1:6379", "tracker:", false).await.unwrap();
            assert_eq!(redis.user_key("7"), "tracker:u:7");
            assert_eq!(redis.torrent_key(&InfoHash([0xab; 20])), format!("tracker:t:{}", "ab".repeat(20)), "the torrent key is unchanged");
        }

        #[test]
        fn test_user_stat_counts_from_entry() {
            let counts = UserStatCounts::from(&user(Some(7), None));
            assert_eq!(counts, UserStatCounts {
                uploaded: 5 << 30,
                downloaded: 300,
                completed: 2,
                active: 1,
                updated: 1_700_000_000,
                torrents_active: 2,
            });
        }

        #[test]
        fn test_memcache_user_format() {
            let counts = UserStatCounts::from(&user(Some(7), None));
            assert_eq!(CacheConnectorMemcache::serialize_user(&counts), "5368709120:300:2:1:1700000000:2");
        }

        #[test]
        fn test_user_cache_id() {
            // The same row identifier the database uses: the uuid when set, otherwise the id.
            assert_eq!(user(Some(7), None).cache_id().as_deref(), Some("7"));
            assert_eq!(user(None, Some("0b6f1c9e")).cache_id().as_deref(), Some("0b6f1c9e"));
            assert_eq!(user(Some(7), Some("0b6f1c9e")).cache_id().as_deref(), Some("0b6f1c9e"));
            assert_eq!(user(None, None).cache_id(), None);
        }
    }
}