use crate::cache::enums::cache_engine::CacheEngine;
use crate::common::structs::custom_error::CustomError;
use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
use crate::config::enums::compression_algorithm::CompressionAlgorithm;
use crate::config::enums::configuration_error::ConfigurationError;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::cache_config::CacheConfig;
use crate::config::structs::configuration::Configuration;
use crate::config::structs::database_config::DatabaseConfig;
use crate::config::structs::database_structure_config::DatabaseStructureConfig;
use crate::config::structs::database_structure_config_blacklist::DatabaseStructureConfigBlacklist;
use crate::config::structs::database_structure_config_keys::DatabaseStructureConfigKeys;
use crate::config::structs::database_structure_config_torrents::DatabaseStructureConfigTorrents;
use crate::config::structs::database_structure_config_users::DatabaseStructureConfigUsers;
use crate::config::structs::database_structure_config_whitelist::DatabaseStructureConfigWhitelist;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::config::structs::sentry_config::SentryConfig;
use crate::config::structs::tracker_config::TrackerConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::security::security::{
    generate_secure_api_key,
    validate_api_key_strength
};
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread::available_parallelism;

impl Configuration {
    pub fn init() -> Configuration {
        Configuration {
            log_level: String::from("info"),
            log_console_interval: 60,
            tracker_config: TrackerConfig {
                api_key: generate_secure_api_key(),
                whitelist_enabled: false,
                blacklist_enabled: false,
                keys_enabled: false,
                keys_cleanup_interval: 60,
                users_enabled: false,
                request_interval: 1800,
                request_interval_minimum: 1800,
                peers_timeout: 2700,
                peers_cleanup_interval: 900,
                peers_cleanup_threads: 256,
                total_downloads: 0,
                swagger: false,
                prometheus_id: String::from("torrust_actix"),
                cluster: ClusterMode::standalone,
                cluster_encoding: ClusterEncoding::binary,
                cluster_token: String::new(),
                cluster_bind_address: String::from("0.0.0.0:8888"),
                cluster_master_address: String::new(),
                cluster_keep_alive: 60,
                cluster_request_timeout: 15,
                cluster_disconnect_timeout: 15,
                cluster_reconnect_interval: 5,
                cluster_max_connections: 25000,
                cluster_threads: available_parallelism().unwrap().get() as u64,
                cluster_ssl: false,
                cluster_ssl_key: String::new(),
                cluster_ssl_cert: String::new(),
                cluster_tls_connection_rate: 256,
                rtc_interval: 30,
                rtc_peers_timeout: 120,
                rtc_compression_enabled: true,
                rtc_compression_algorithm: CompressionAlgorithm::Lz4,
                rtc_compression_level: 1,
            },
            sentry_config: SentryConfig {
                enabled: false,
                dsn: String::new(),
                debug: false,
                sample_rate: 1.0,
                max_breadcrumbs: 100,
                attach_stacktrace: true,
                send_default_pii: false,
                traces_sample_rate: 1.0,
            },
            database: DatabaseConfig {
                engine: DatabaseDrivers::sqlite3,
                path: String::from("sqlite://data.db"),
                persistent: false,
                persistent_interval: 60,
                insert_vacant: false,
                remove_action: false,
                update_completed: true,
                update_peers: false,
            },
            cache: None,
            database_structure: DatabaseStructureConfig {
                torrents: DatabaseStructureConfigTorrents {
                    table_name: String::from("torrents"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                    column_seeds: String::from("seeds"),
                    column_peers: String::from("peers"),
                    column_completed: String::from("completed"),
                    persistent: None
                },
                whitelist: DatabaseStructureConfigWhitelist {
                    table_name: String::from("whitelist"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                    persistent: None
                },
                blacklist: DatabaseStructureConfigBlacklist {
                    table_name: String::from("blacklist"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                    persistent: None
                },
                keys: DatabaseStructureConfigKeys {
                    table_name: String::from("keys"),
                    column_hash: String::from("hash"),
                    bin_type_hash: true,
                    column_timeout: String::from("timeout"),
                    persistent: None
                },
                users: DatabaseStructureConfigUsers {
                    table_name: String::from("users"),
                    id_uuid: true,
                    column_uuid: String::from("uuid"),
                    column_id: "id".to_string(),
                    column_active: String::from("active"),
                    column_key: String::from("key"),
                    bin_type_key: true,
                    column_uploaded: String::from("uploaded"),
                    column_downloaded: String::from("downloaded"),
                    column_completed: String::from("completed"),
                    column_updated: String::from("updated"),
                    persistent: None
                }
            },
            http_server: vec!(
                HttpTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:6969"),
                    real_ip: String::from("X-Real-IP"),
                    trusted_proxies: false,
                    keep_alive: 60,
                    request_timeout: 15,
                    disconnect_timeout: 15,
                    max_connections: 25000,
                    threads: available_parallelism().unwrap().get() as u64,
                    ssl: false,
                    ssl_key: String::new(),
                    ssl_cert: String::new(),
                    tls_connection_rate: 256,
                    rtctorrent: false
                }
            ),
            udp_server: vec!(
                UdpTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:6969"),
                    udp_threads: 2,
                    worker_threads: available_parallelism().unwrap().get(),
                    receive_buffer_size: 134_217_728,
                    send_buffer_size: 67_108_864,
                    reuse_address: true,
                    use_payload_ip: false,
                    simple_proxy_protocol: false,
                }
            ),
            api_server: vec!(
                ApiTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:8080"),
                    real_ip: String::from("X-Real-IP"),
                    trusted_proxies: false,
                    keep_alive: 60,
                    request_timeout: 30,
                    disconnect_timeout: 30,
                    max_connections: 25000,
                    threads: available_parallelism().unwrap().get() as u64,
                    ssl: false,
                    ssl_key: String::new(),
                    ssl_cert: String::new(),
                    tls_connection_rate: 256
                }
            ),
        }
    }

    pub fn env_overrides(config: &mut Configuration) -> &mut Configuration {
        if let Ok(value) = env::var("LOG_LEVEL") { config.log_level = value; }
        if let Ok(value) = env::var("LOG_CONSOLE_INTERVAL") { config.log_console_interval = value.parse::<u64>().unwrap_or(60u64); }
        if let Ok(value) = env::var("TRACKER__API_KEY") {
            config.tracker_config.api_key = value;
        }
        if let Ok(value) = env::var("TRACKER__WHITELIST_ENABLED") {
            config.tracker_config.whitelist_enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("TRACKER__BLACKLIST_ENABLED") {
            config.tracker_config.blacklist_enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("TRACKER__KEYS_ENABLED") {
            config.tracker_config.keys_enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("TRACKER__USERS_ENABLED") {
            config.tracker_config.users_enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("TRACKER__SWAGGER") {
            config.tracker_config.swagger = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("TRACKER__KEYS_CLEANUP_INTERVAL") {
            config.tracker_config.keys_cleanup_interval = value.parse::<u64>().unwrap_or(60u64);
        }
        if let Ok(value) = env::var("TRACKER__REQUEST_INTERVAL") {
            config.tracker_config.request_interval = value.parse::<u64>().unwrap_or(1800u64);
        }
        if let Ok(value) = env::var("TRACKER__REQUEST_INTERVAL_MINIMUM") {
            config.tracker_config.request_interval_minimum = value.parse::<u64>().unwrap_or(1800u64);
        }
        if let Ok(value) = env::var("TRACKER__PEERS_TIMEOUT") {
            config.tracker_config.peers_timeout = value.parse::<u64>().unwrap_or(2700u64);
        }
        if let Ok(value) = env::var("TRACKER__PEERS_CLEANUP_INTERVAL") {
            config.tracker_config.peers_cleanup_interval = value.parse::<u64>().unwrap_or(900u64);
        }
        if let Ok(value) = env::var("TRACKER__PEERS_CLEANUP_THREADS") {
            config.tracker_config.peers_cleanup_threads = value.parse::<u64>().unwrap_or(256u64);
        }
        if let Ok(value) = env::var("TRACKER__PROMETHEUS_ID") {
            config.tracker_config.prometheus_id = value;
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER") {
            config.tracker_config.cluster = match value.as_str() {
                "standalone" => ClusterMode::standalone,
                "master" => ClusterMode::master,
                "slave" => ClusterMode::slave,
                _ => ClusterMode::standalone
            };
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_ENCODING") {
            config.tracker_config.cluster_encoding = match value.as_str() {
                "binary" => ClusterEncoding::binary,
                "json" => ClusterEncoding::json,
                "msgpack" => ClusterEncoding::msgpack,
                _ => ClusterEncoding::binary
            };
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_TOKEN") {
            config.tracker_config.cluster_token = value;
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_BIND_ADDRESS") {
            config.tracker_config.cluster_bind_address = value;
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_MASTER_ADDRESS") {
            config.tracker_config.cluster_master_address = value;
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_KEEP_ALIVE") {
            config.tracker_config.cluster_keep_alive = value.parse::<u64>().unwrap_or(60u64);
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_REQUEST_TIMEOUT") {
            config.tracker_config.cluster_request_timeout = value.parse::<u64>().unwrap_or(15u64);
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_DISCONNECT_TIMEOUT") {
            config.tracker_config.cluster_disconnect_timeout = value.parse::<u64>().unwrap_or(15u64);
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_RECONNECT_INTERVAL") {
            config.tracker_config.cluster_reconnect_interval = value.parse::<u64>().unwrap_or(5u64);
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_MAX_CONNECTIONS") {
            config.tracker_config.cluster_max_connections = value.parse::<u64>().unwrap_or(25000u64);
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_THREADS") {
            config.tracker_config.cluster_threads = value.parse::<u64>().unwrap_or(available_parallelism().unwrap().get() as u64);
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_SSL") {
            config.tracker_config.cluster_ssl = match value.as_str() { "true" => true, "false" => false, _ => false };
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_SSL_KEY") {
            config.tracker_config.cluster_ssl_key = value;
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_SSL_CERT") {
            config.tracker_config.cluster_ssl_cert = value;
        }
        if let Ok(value) = env::var("TRACKER__CLUSTER_TLS_CONNECTION_RATE") {
            config.tracker_config.cluster_tls_connection_rate = value.parse::<u64>().unwrap_or(256u64);
        }
        if let Ok(value) = env::var("TRACKER__RTC_INTERVAL") {
            config.tracker_config.rtc_interval = value.parse::<u64>().unwrap_or(10u64);
        }
        if let Ok(value) = env::var("TRACKER__RTC_PEERS_TIMEOUT") {
            config.tracker_config.rtc_peers_timeout = value.parse::<u64>().unwrap_or(120u64);
        }
        if let Ok(value) = env::var("TRACKER__TOTAL_DOWNLOADS") {
            config.tracker_config.total_downloads = value.parse::<u64>().unwrap_or(0u64);
        }
        if let Ok(value) = env::var("TRACKER__RTC_COMPRESSION_ENABLED") {
            config.tracker_config.rtc_compression_enabled = match value.as_str() { "true" => true, "false" => false, _ => true };
        }
        if let Ok(value) = env::var("TRACKER__RTC_COMPRESSION_ALGORITHM") {
            config.tracker_config.rtc_compression_algorithm = match value.to_lowercase().as_str() {
                "zstd" => CompressionAlgorithm::Zstd,
                _ => CompressionAlgorithm::Lz4,
            };
        }
        if let Ok(value) = env::var("TRACKER__RTC_COMPRESSION_LEVEL") {
            config.tracker_config.rtc_compression_level = value.parse::<u32>().unwrap_or(1u32);
        }
        if let Ok(value) = env::var("SENTRY__ENABLED") {
            config.sentry_config.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("SENTRY__DEBUG") {
            config.sentry_config.debug = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("SENTRY__ATTACH_STACKTRACE") {
            config.sentry_config.attach_stacktrace = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("SENTRY__SEND_DEFAULT_PII") {
            config.sentry_config.send_default_pii = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("SENTRY__DSN") {
            config.sentry_config.dsn = value;
        }
        if let Ok(value) = env::var("SENTRY__MAX_BREADCRUMBS") {
            config.sentry_config.max_breadcrumbs = value.parse::<usize>().unwrap_or(100);
        }
        if let Ok(value) = env::var("SENTRY__SAMPLE_RATE") {
            config.sentry_config.sample_rate = value.parse::<f32>().unwrap_or(1.0);
        }
        if let Ok(value) = env::var("SENTRY__TRACES_SAMPLE_RATE") {
            config.sentry_config.traces_sample_rate = value.parse::<f32>().unwrap_or(1.0);
        }
        if let Ok(value) = env::var("DATABASE__PERSISTENT") {
            config.database.persistent = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("DATABASE__INSERT_VACANT") {
            config.database.insert_vacant = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("DATABASE__REMOVE_ACTION") {
            config.database.remove_action = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("DATABASE__UPDATE_COMPLETED") {
            config.database.update_completed = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE__UPDATE_PEERS") {
            config.database.update_peers = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
        }
        if let Ok(value) = env::var("DATABASE__PATH") {
            config.database.path = value;
        }
        if let Ok(value) = env::var("DATABASE__ENGINE") {
            config.database.engine = match value.as_str() {
                "sqlite3" => { DatabaseDrivers::sqlite3 }
                "mysql" => { DatabaseDrivers::mysql }
                "pgsql" => { DatabaseDrivers::pgsql }
                _ => { DatabaseDrivers::sqlite3 }
            };
        }
        if let Ok(value) = env::var("DATABASE__PERSISTENT_INTERVAL") {
            config.database.persistent_interval = value.parse::<u64>().unwrap_or(60u64);
        }
        if let Ok(value) = env::var("CACHE__ENABLED") {
            let enabled = match value.as_str() { "true" => true, "false" => false, _ => false };
            if let Some(ref mut cache) = config.cache {
                cache.enabled = enabled;
            } else if enabled {
                config.cache = Some(CacheConfig::default());
                config.cache.as_mut().unwrap().enabled = enabled;
            }
        }
        if let Ok(value) = env::var("CACHE__ENGINE") {
            let engine = match value.as_str() {
                "redis" => CacheEngine::redis,
                "memcache" => CacheEngine::memcache,
                _ => CacheEngine::redis
            };
            if let Some(ref mut cache) = config.cache {
                cache.engine = engine;
            } else {
                config.cache = Some(CacheConfig {
                    engine,
                    ..Default::default()
                });
            }
        }
        if let Ok(value) = env::var("CACHE__ADDRESS") {
            if let Some(ref mut cache) = config.cache {
                cache.address = value;
            } else {
                config.cache = Some(CacheConfig {
                    address: value,
                    ..Default::default()
                });
            }
        }
        if let Ok(value) = env::var("CACHE__PREFIX") {
            if let Some(ref mut cache) = config.cache {
                cache.prefix = value;
            } else {
                config.cache = Some(CacheConfig {
                    prefix: value,
                    ..Default::default()
                });
            }
        }
        if let Ok(value) = env::var("CACHE__TTL") {
            let ttl = value.parse::<u64>().unwrap_or(300u64);
            if let Some(ref mut cache) = config.cache {
                cache.ttl = ttl;
            } else {
                config.cache = Some(CacheConfig {
                    ttl,
                    ..Default::default()
                });
            }
        }
        if let Ok(value) = env::var("CACHE__SPLIT_PEERS") {
            let split_peers = matches!(value.as_str(), "true");
            if let Some(ref mut cache) = config.cache {
                cache.split_peers = split_peers;
            } else {
                config.cache = Some(CacheConfig {
                    split_peers,
                    ..Default::default()
                });
            }
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__BIN_TYPE_INFOHASH") {
            config.database_structure.torrents.bin_type_infohash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__TABLE_NAME") {
            config.database_structure.torrents.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__COLUMN_INFOHASH") {
            config.database_structure.torrents.column_infohash = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__COLUMN_SEEDS") {
            config.database_structure.torrents.column_seeds = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__COLUMN_PEERS") {
            config.database_structure.torrents.column_peers = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__COLUMN_COMPLETED") {
            config.database_structure.torrents.column_completed = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__TORRENTS__PERSISTENT") {
            config.database_structure.torrents.persistent = match value.as_str() { "true" => Some(true), "false" => Some(false), _ => None };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH") {
            config.database_structure.whitelist.bin_type_infohash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__TABLE_NAME") {
            config.database_structure.whitelist.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH") {
            config.database_structure.whitelist.column_infohash = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__PERSISTENT") {
            config.database_structure.whitelist.persistent = match value.as_str() { "true" => Some(true), "false" => Some(false), _ => None };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__BIN_TYPE_INFOHASH") {
            config.database_structure.blacklist.bin_type_infohash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__TABLE_NAME") {
            config.database_structure.blacklist.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__COLUMN_INFOHASH") {
            config.database_structure.blacklist.column_infohash = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__PERSISTENT") {
            config.database_structure.blacklist.persistent = match value.as_str() { "true" => Some(true), "false" => Some(false), _ => None };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__KEYS__BIN_TYPE_HASH") {
            config.database_structure.keys.bin_type_hash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__KEYS__TABLE_NAME") {
            config.database_structure.keys.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__KEYS__COLUMN_HASH") {
            config.database_structure.keys.column_hash = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__KEYS__COLUMN_TIMEOUT") {
            config.database_structure.keys.column_timeout = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__KEYS__PERSISTENT") {
            config.database_structure.keys.persistent = match value.as_str() { "true" => Some(true), "false" => Some(false), _ => None };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__ID_UUID") {
            config.database_structure.users.id_uuid = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__BIN_TYPE_KEY") {
            config.database_structure.users.bin_type_key = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__TABLE_NAME") {
            config.database_structure.users.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_UUID") {
            config.database_structure.users.column_uuid = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_ID") {
            config.database_structure.users.column_id = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_ACTIVE") {
            config.database_structure.users.column_active = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_KEY") {
            config.database_structure.users.column_key = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_UPLOADED") {
            config.database_structure.users.column_uploaded = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_DOWNLOADED") {
            config.database_structure.users.column_downloaded = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_COMPLETED") {
            config.database_structure.users.column_completed = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__COLUMN_UPDATED") {
            config.database_structure.users.column_updated = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__USERS__PERSISTENT") {
            config.database_structure.users.persistent = match value.as_str() { "true" => Some(true), "false" => Some(false), _ => None };
        }
        let mut api_iteration = 0;
        loop {
            match config.api_server.get_mut(api_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("API_{api_iteration}_ENABLED")) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_SSL")) {
                        block.ssl = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_BIND_ADDRESS")) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_REAL_IP")) {
                        block.real_ip = value;
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_SSL_KEY")) {
                        block.ssl_key = value;
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_SSL_CERT")) {
                        block.ssl_cert = value;
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_KEEP_ALIVE")) {
                        block.keep_alive = value.parse::<u64>().unwrap_or(60);
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_REQUEST_TIMEOUT")) {
                        block.request_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_DISCONNECT_TIMEOUT")) {
                        block.disconnect_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_MAX_CONNECTIONS")) {
                        block.max_connections = value.parse::<u64>().unwrap_or(25000);
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_THREADS")) {
                        block.threads = value.parse::<u64>().unwrap_or(available_parallelism().unwrap().get() as u64);
                    }
                    if let Ok(value) = env::var(format!("API_{api_iteration}_TLS_CONNECTION_RATE")) {
                        block.tls_connection_rate = value.parse::<u64>().unwrap_or(256);
                    }
                }
            }
            api_iteration += 1;
        }
        let mut http_iteration = 0;
        loop {
            match config.http_server.get_mut(http_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_ENABLED")) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_SSL")) {
                        block.ssl = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_BIND_ADDRESS")) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_REAL_IP")) {
                        block.real_ip = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_SSL_KEY")) {
                        block.ssl_key = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_SSL_CERT")) {
                        block.ssl_cert = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_KEEP_ALIVE")) {
                        block.keep_alive = value.parse::<u64>().unwrap_or(60);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_REQUEST_TIMEOUT")) {
                        block.request_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_DISCONNECT_TIMEOUT")) {
                        block.disconnect_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_MAX_CONNECTIONS")) {
                        block.max_connections = value.parse::<u64>().unwrap_or(25000);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_THREADS")) {
                        block.threads = value.parse::<u64>().unwrap_or(available_parallelism().unwrap().get() as u64);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_TLS_CONNECTION_RATE")) {
                        block.tls_connection_rate = value.parse::<u64>().unwrap_or(256);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{http_iteration}_RTCTORRENT")) {
                        block.rtctorrent = match value.as_str() { "true" => true, "false" => false, _ => false };
                    }
                }
            }
            http_iteration += 1;
        }
        let mut udp_iteration = 0;
        loop {
            match config.udp_server.get_mut(udp_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_ENABLED")) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_BIND_ADDRESS")) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_UDP_THREADS")) {
                        block.udp_threads = value.parse::<usize>().unwrap_or(2);
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_WORKER_THREADS")) {
                        block.worker_threads = value.parse::<usize>().unwrap_or(available_parallelism().unwrap().get());
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_RECEIVE_BUFFER_SIZE")) {
                        block.receive_buffer_size = value.parse::<usize>().unwrap_or(134_217_728);
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_SEND_BUFFER_SIZE")) {
                        block.send_buffer_size = value.parse::<usize>().unwrap_or(67_108_864);
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_REUSE_ADDRESS")) {
                        block.reuse_address = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_USE_PAYLOAD_IP")) {
                        block.use_payload_ip = match value.as_str() { "true" => true, "false" => false, _ => false };
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_SIMPLE_PROXY_PROTOCOL")) {
                        block.simple_proxy_protocol = match value.as_str() { "true" => true, "false" => false, _ => false };
                    }
                }
            }
            udp_iteration += 1;
        }
        config
    }

    pub fn save_file(path: &str, data: String) -> Result<(), ConfigurationError> {
        match File::create(path) {
            Ok(mut file) => {
                match file.write_all(data.as_ref()) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(ConfigurationError::IOError(e))
                }
            }
            Err(e) => Err(ConfigurationError::IOError(e))
        }
    }

    pub fn save_from_config(config: Arc<Configuration>, path: &str)
    {
        let config_toml = toml::to_string(&config).unwrap();
        match Self::save_file(path, config_toml) {
            Ok(()) => { eprintln!("[CONFIG SAVE] Config file is saved"); }
            Err(_) => { eprintln!("[CONFIG SAVE] Unable to save to {path}"); }
        }
    }

    pub fn load(data: &[u8]) -> Result<Configuration, toml::de::Error> {
        toml::from_str(&String::from_utf8_lossy(data))
    }

    pub fn load_file(path: &str) -> Result<Configuration, ConfigurationError> {
        match std::fs::read(path) {
            Err(e) => Err(ConfigurationError::IOError(e)),
            Ok(data) => {
                match Self::load(data.as_slice()) {
                    Ok(cfg) => {
                        Ok(cfg)
                    }
                    Err(e) => Err(ConfigurationError::ParseError(e)),
                }
            }
        }
    }

    pub fn load_from_file(create: bool) -> Result<Configuration, CustomError> {
        let mut config = Configuration::init();
        match Configuration::load_file("config.toml") {
            Ok(c) => { config = c; }
            Err(error) => {
                eprintln!("No config file found or corrupt.");
                eprintln!("[ERROR] {error}");
                if !create {
                    eprintln!("You can either create your own config.toml file, or start this app using '--create-config' as parameter.");
                    return Err(CustomError::new("will not create automatically config.toml file"));
                }
                eprintln!("Creating config file..");
                let config_toml = Self::generate_annotated_config(&config);
                let save_file = Configuration::save_file("config.toml", config_toml);
                return match save_file {
                    Ok(()) => {
                        eprintln!("Please edit the config.TOML in the root folder, exiting now...");
                        Err(CustomError::new("create config.toml file"))
                    }
                    Err(e) => {
                        eprintln!("config.toml file could not be created, check permissions...");
                        eprintln!("{e}");
                        Err(CustomError::new("could not create config.toml file"))
                    }
                };
            }
        }
        Self::env_overrides(&mut config);
        println!("[VALIDATE] Validating configuration...");
        Self::validate(config.clone());
        Ok(config)
    }

    pub fn validate(config: Configuration) {
        if validate_api_key_strength(&config.tracker_config.api_key) {
            println!("[VALIDATE] API key strength: OK");
        } else {
            eprintln!("[SECURITY WARNING] API key is weak! Please use a stronger API key.");
            eprintln!("[SECURITY WARNING] Generate a secure key with: 'head -c 32 /dev/urandom | base64'");
        }
        let check_map = vec![
            ("[TRACKER_CONFIG] prometheus_id", config.tracker_config.clone().prometheus_id, r"^[a-zA-Z0-9_]+$".to_string()),
            ("[DB: torrents]", config.database_structure.clone().torrents.table_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: infohash", config.database_structure.clone().torrents.column_infohash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: seeds", config.database_structure.clone().torrents.column_seeds, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: peers", config.database_structure.clone().torrents.column_peers, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: completed", config.database_structure.clone().torrents.column_completed, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: whitelist]", config.database_structure.clone().whitelist.table_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: whitelist] Column: infohash", config.database_structure.clone().whitelist.column_infohash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: blacklist]", config.database_structure.clone().blacklist.table_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: blacklist] Column: infohash", config.database_structure.clone().blacklist.column_infohash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: keys]", config.database_structure.clone().keys.table_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: keys] Column: hash", config.database_structure.clone().keys.column_hash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: keys] Column: timeout", config.database_structure.clone().keys.column_timeout, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users]", config.database_structure.clone().users.table_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: id", config.database_structure.clone().users.column_id, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: uuid", config.database_structure.clone().users.column_uuid, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: key", config.database_structure.clone().users.column_key, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: uploaded", config.database_structure.clone().users.column_uploaded, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: downloaded", config.database_structure.clone().users.column_downloaded, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: completed", config.database_structure.clone().users.column_completed, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: active", config.database_structure.clone().users.column_active, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: updated", config.database_structure.clone().users.column_updated, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
        ];
        for (name, value, regex) in check_map {
            Self::validate_value(name, value, regex);
        }
        for (index, api_server) in config.api_server.iter().enumerate() {
            if api_server.enabled {
                Self::validate_socket_address(
                    &format!("api_server[{index}].bind_address"),
                    &api_server.bind_address,
                );
                if api_server.ssl {
                    assert!(!api_server.ssl_key.is_empty(), "[VALIDATE CONFIG] api_server[{index}] ssl=true but ssl_key is not set");
                    assert!(!api_server.ssl_cert.is_empty(), "[VALIDATE CONFIG] api_server[{index}] ssl=true but ssl_cert is not set");
                }
            }
        }
        for (index, http_server) in config.http_server.iter().enumerate() {
            if http_server.enabled {
                Self::validate_socket_address(
                    &format!("http_server[{index}].bind_address"),
                    &http_server.bind_address,
                );
                println!(
                    "[VALIDATE] http_server[{}] rtctorrent: {}",
                    index,
                    if http_server.rtctorrent { "enabled" } else { "disabled" }
                );
                if http_server.ssl {
                    assert!(!http_server.ssl_key.is_empty(), "[VALIDATE CONFIG] http_server[{index}] ssl=true but ssl_key is not set");
                    assert!(!http_server.ssl_cert.is_empty(), "[VALIDATE CONFIG] http_server[{index}] ssl=true but ssl_cert is not set");
                }
            }
        }
        for (index, udp_server) in config.udp_server.iter().enumerate() {
            if udp_server.enabled {
                Self::validate_socket_address(
                    &format!("udp_server[{index}].bind_address"),
                    &udp_server.bind_address,
                );
                assert!(udp_server.udp_threads > 0, "[VALIDATE CONFIG] udp_server[{index}] udp_threads must be > 0");
                assert!(udp_server.worker_threads > 0, "[VALIDATE CONFIG] udp_server[{index}] worker_threads must be > 0");
            }
        }
        Self::validate_tracker(&config);
        Self::validate_sentry(&config);
        Self::validate_cluster(&config);
        Self::validate_cache(&config);
        Self::validate_compression(&config);
    }

    pub fn validate_tracker(config: &Configuration) {
        let tc = &config.tracker_config;
        assert!(tc.request_interval > 0, "[VALIDATE CONFIG] request_interval must be > 0");
        assert!(tc.request_interval_minimum > 0, "[VALIDATE CONFIG] request_interval_minimum must be > 0");
        assert!(tc.request_interval_minimum <= tc.request_interval, "[VALIDATE CONFIG] request_interval_minimum ({}) must be <= request_interval ({})", tc.request_interval_minimum, tc.request_interval);
        assert!(tc.peers_timeout > 0, "[VALIDATE CONFIG] peers_timeout must be > 0");
        assert!(tc.peers_cleanup_interval > 0, "[VALIDATE CONFIG] peers_cleanup_interval must be > 0");
        assert!(tc.peers_cleanup_threads > 0, "[VALIDATE CONFIG] peers_cleanup_threads must be > 0");
        assert!(tc.keys_cleanup_interval > 0, "[VALIDATE CONFIG] keys_cleanup_interval must be > 0");
        assert!(tc.cluster_threads > 0, "[VALIDATE CONFIG] cluster_threads must be > 0");
        assert!(tc.rtc_interval > 0, "[VALIDATE CONFIG] rtc_interval must be > 0");
        assert!(tc.rtc_peers_timeout > 0, "[VALIDATE CONFIG] rtc_peers_timeout must be > 0");
        println!("[VALIDATE] request_interval: {}s (min: {}s)", tc.request_interval, tc.request_interval_minimum);
        println!("[VALIDATE] peers_timeout: {}s, cleanup_interval: {}s, cleanup_threads: {}", tc.peers_timeout, tc.peers_cleanup_interval, tc.peers_cleanup_threads);
        println!("[VALIDATE] rtc_interval: {}s, rtc_peers_timeout: {}s", tc.rtc_interval, tc.rtc_peers_timeout);
    }

    pub fn validate_sentry(config: &Configuration) {
        let sc = &config.sentry_config;
        if sc.enabled {
            assert!(!sc.dsn.is_empty(), "[VALIDATE CONFIG] sentry.enabled=true but sentry.dsn is not set");
            println!("[VALIDATE] Sentry enabled: dsn configured, sample_rate={}, traces_sample_rate={}", sc.sample_rate, sc.traces_sample_rate);
        } else {
            println!("[VALIDATE] Sentry: disabled");
        }
    }

    pub fn validate_cache(config: &Configuration) {
        if let Some(ref cache) = config.cache {
            if cache.enabled {
                println!("[VALIDATE] Cache enabled: {}", cache.engine);
                Self::validate_socket_address("cache.address", &cache.address);
                println!("[VALIDATE] Cache prefix: {}", cache.prefix);
                println!("[VALIDATE] Cache TTL: {} seconds", cache.ttl);
                println!("[VALIDATE] Cache split_peers: {}", cache.split_peers);
            } else {
                println!("[VALIDATE] Cache: disabled");
            }
        } else {
            println!("[VALIDATE] Cache: not configured");
        }
    }

    pub fn validate_compression(config: &Configuration) {
        let tc = &config.tracker_config;
        if tc.rtc_compression_enabled {
            println!("[VALIDATE] RTC compression enabled: algorithm={:?}, level={}", tc.rtc_compression_algorithm, tc.rtc_compression_level);
            if let CompressionAlgorithm::Zstd = tc.rtc_compression_algorithm {
                assert!(
                    tc.rtc_compression_level >= 1 && tc.rtc_compression_level <= 22,
                    "[VALIDATE CONFIG] rtc_compression_level must be between 1 and 22 for zstd (got {})",
                    tc.rtc_compression_level
                );
            }
        } else {
            println!("[VALIDATE] RTC compression: disabled");
        }
    }

    pub fn validate_cluster(config: &Configuration) {
        match config.tracker_config.cluster {
            ClusterMode::standalone => {
                println!("[VALIDATE] Cluster mode: standalone");
            }
            ClusterMode::master => {
                println!("[VALIDATE] Cluster mode: master");
                assert!(!config.tracker_config.cluster_token.is_empty(), "[VALIDATE CONFIG] Cluster mode 'master' requires 'cluster_token' to be set for authentication");
                if !config.tracker_config.cluster_ssl {
                    eprintln!("[VALIDATE WARNING] Cluster SSL is disabled - cluster_token will be transmitted in plaintext!");
                }
                assert!(!config.tracker_config.cluster_bind_address.is_empty(), "[VALIDATE CONFIG] Cluster mode 'master' requires 'cluster_bind_address' to be set");
                Self::validate_socket_address("cluster_bind_address", &config.tracker_config.cluster_bind_address);
                if config.tracker_config.cluster_ssl {
                    assert!(!config.tracker_config.cluster_ssl_key.is_empty(), "[VALIDATE CONFIG] Cluster SSL enabled but 'cluster_ssl_key' is not set");
                    assert!(!config.tracker_config.cluster_ssl_cert.is_empty(), "[VALIDATE CONFIG] Cluster SSL enabled but 'cluster_ssl_cert' is not set");
                }
                println!("[VALIDATE] Cluster encoding: {:?}", config.tracker_config.cluster_encoding);
                println!("[VALIDATE] Cluster bind address: {}", config.tracker_config.cluster_bind_address);
                println!("[VALIDATE] Cluster SSL: {}", if config.tracker_config.cluster_ssl { "wss" } else { "ws" });
            }
            ClusterMode::slave => {
                println!("[VALIDATE] Cluster mode: slave");
                assert!(!config.tracker_config.cluster_token.is_empty(), "[VALIDATE CONFIG] Cluster mode 'slave' requires 'cluster_token' to be set for authentication");
                assert!(!config.tracker_config.cluster_master_address.is_empty(), "[VALIDATE CONFIG] Cluster mode 'slave' requires 'cluster_master_address' to be set");
                Self::validate_socket_address("cluster_master_address", &config.tracker_config.cluster_master_address);
                println!("[VALIDATE] Cluster master address: {}", config.tracker_config.cluster_master_address);
                println!("[VALIDATE] Cluster SSL: {}", if config.tracker_config.cluster_ssl { "wss" } else { "ws" });
            }
        }
    }
    
    pub fn validate_socket_address(field_name: &str, address: &str) {
        use std::net::SocketAddr;
        match address.parse::<SocketAddr>() {
            Ok(addr) => {
                println!("[VALIDATE] {field_name} is valid: {addr}");
            }
            Err(e) => {
                panic!(
                    "[VALIDATE CONFIG] '{field_name}' has invalid format: '{address}'. \
                    Expected IPv4 format '1.2.3.4:8888' or IPv6 format '[2a00:1768:1001:0026::0183]:80'. \
                    Error: {e}"
                );
            }
        }
    }

    pub fn validate_value(name: &str, value: String, regex: String)
    {
        let regex_check = Regex::new(regex.as_str()).unwrap();
        assert!(regex_check.is_match(value.as_str()), "[VALIDATE CONFIG] Error checking {name} [:] Name: \"{value}\" [:] Regex: \"{regex_check}\"");
    }

    /// Generate a `config.toml` string with `# Optional:` remarks injected
    /// above every key that may be omitted from the file.  Called by
    /// `--create-config` so new users know which fields are required.
    pub fn generate_annotated_config(config: &Configuration) -> String {
        let raw = toml::to_string(config).unwrap();
        Self::annotate_config_toml(&raw)
    }

    fn annotate_config_toml(toml: &str) -> String {
        type Remarks<'a> = std::collections::HashMap<(&'a str, &'a str), &'a str>;
        let mut remarks: Remarks = Remarks::new();
        remarks.insert(("tracker_config", "whitelist_enabled"), "# Optional: defaults to false -- enable whitelist-only tracking");
        remarks.insert(("tracker_config", "blacklist_enabled"), "# Optional: defaults to false -- enable blacklist filtering");
        remarks.insert(("tracker_config", "keys_enabled"), "# Optional: defaults to false -- require announce keys");
        remarks.insert(("tracker_config", "keys_cleanup_interval"), "# Optional: defaults to 60 -- expired-key cleanup interval (seconds)");
        remarks.insert(("tracker_config", "users_enabled"), "# Optional: defaults to false -- enable per-user statistics");
        remarks.insert(("tracker_config", "swagger"), "# Optional: defaults to false -- expose Swagger UI at <api>/swagger-ui/");
        remarks.insert(("tracker_config", "prometheus_id"), "# Optional: defaults to \"torrust_actix\" -- Prometheus metric label");
        remarks.insert(("tracker_config", "cluster"), "# Optional: defaults to \"standalone\" -- cluster mode: standalone | master | slave");
        remarks.insert(("tracker_config", "cluster_encoding"), "# Optional: defaults to \"binary\" -- cluster wire format: binary | json | msgpack");
        remarks.insert(("tracker_config", "cluster_token"), "# Optional: defaults to \"\" -- required for master/slave modes");
        remarks.insert(("tracker_config", "cluster_bind_address"), "# Optional: defaults to \"0.0.0.0:8888\" -- required for master mode");
        remarks.insert(("tracker_config", "cluster_master_address"), "# Optional: defaults to \"\" -- required for slave mode");
        remarks.insert(("tracker_config", "cluster_keep_alive"), "# Optional: defaults to 60 -- WebSocket keep-alive interval (seconds)");
        remarks.insert(("tracker_config", "cluster_request_timeout"), "# Optional: defaults to 15 -- cluster request timeout (seconds)");
        remarks.insert(("tracker_config", "cluster_disconnect_timeout"), "# Optional: defaults to 15 -- cluster disconnect timeout (seconds)");
        remarks.insert(("tracker_config", "cluster_reconnect_interval"), "# Optional: defaults to 5 -- slave reconnect interval (seconds)");
        remarks.insert(("tracker_config", "cluster_max_connections"), "# Optional: defaults to 25000 -- max simultaneous cluster connections");
        remarks.insert(("tracker_config", "cluster_threads"), "# Optional: defaults to CPU core count -- cluster I/O worker threads");
        remarks.insert(("tracker_config", "cluster_ssl"), "# Optional: defaults to false -- enable TLS for cluster WebSocket");
        remarks.insert(("tracker_config", "cluster_ssl_key"), "# Optional: defaults to \"\" -- required when cluster_ssl = true");
        remarks.insert(("tracker_config", "cluster_ssl_cert"), "# Optional: defaults to \"\" -- required when cluster_ssl = true");
        remarks.insert(("tracker_config", "cluster_tls_connection_rate"), "# Optional: defaults to 256 -- max new TLS cluster connections per second");
        remarks.insert(("tracker_config", "rtc_interval"), "# Optional: defaults to 30 -- RtcTorrent signalling poll interval (seconds)");
        remarks.insert(("tracker_config", "rtc_peers_timeout"), "# Optional: defaults to 120 -- RtcTorrent peer inactivity timeout (seconds)");
        remarks.insert(("tracker_config", "rtc_compression_enabled"), "# Optional: defaults to true -- compress RTC SDP strings in memory");
        remarks.insert(("tracker_config", "rtc_compression_algorithm"), "# Optional: defaults to \"lz4\" -- RTC compression algorithm: lz4 | zstd");
        remarks.insert(("tracker_config", "rtc_compression_level"), "# Optional: defaults to 1 -- compression level (Zstd: 1-22; LZ4: ignored)");
        remarks.insert(("sentry_config", "enabled"), "# Optional: defaults to false -- enable Sentry error tracking");
        remarks.insert(("sentry_config", "dsn"), "# Optional: required when enabled = true");
        remarks.insert(("sentry_config", "debug"), "# Optional: defaults to false");
        remarks.insert(("sentry_config", "sample_rate"), "# Optional: defaults to 1.0");
        remarks.insert(("sentry_config", "max_breadcrumbs"), "# Optional: defaults to 100");
        remarks.insert(("sentry_config", "attach_stacktrace"), "# Optional: defaults to true");
        remarks.insert(("sentry_config", "send_default_pii"), "# Optional: defaults to false");
        remarks.insert(("sentry_config", "traces_sample_rate"), "# Optional: defaults to 1.0");
        remarks.insert(("database_structure.torrents", "table_name"), "# Optional: defaults to \"torrents\"");
        remarks.insert(("database_structure.torrents", "column_infohash"), "# Optional: defaults to \"infohash\"");
        remarks.insert(("database_structure.torrents", "bin_type_infohash"), "# Optional: defaults to true");
        remarks.insert(("database_structure.torrents", "column_seeds"), "# Optional: defaults to \"seeds\"");
        remarks.insert(("database_structure.torrents", "column_peers"), "# Optional: defaults to \"peers\"");
        remarks.insert(("database_structure.torrents", "column_completed"), "# Optional: defaults to \"completed\"");
        remarks.insert(("database_structure.whitelist", "table_name"), "# Optional: defaults to \"whitelist\"");
        remarks.insert(("database_structure.whitelist", "column_infohash"), "# Optional: defaults to \"infohash\"");
        remarks.insert(("database_structure.whitelist", "bin_type_infohash"), "# Optional: defaults to true");
        remarks.insert(("database_structure.blacklist", "table_name"), "# Optional: defaults to \"blacklist\"");
        remarks.insert(("database_structure.blacklist", "column_infohash"), "# Optional: defaults to \"infohash\"");
        remarks.insert(("database_structure.blacklist", "bin_type_infohash"), "# Optional: defaults to true");
        remarks.insert(("database_structure.keys", "table_name"), "# Optional: defaults to \"keys\"");
        remarks.insert(("database_structure.keys", "column_hash"), "# Optional: defaults to \"hash\"");
        remarks.insert(("database_structure.keys", "bin_type_hash"), "# Optional: defaults to true");
        remarks.insert(("database_structure.keys", "column_timeout"), "# Optional: defaults to \"timeout\"");
        remarks.insert(("database_structure.users", "table_name"), "# Optional: defaults to \"users\"");
        remarks.insert(("database_structure.users", "id_uuid"), "# Optional: defaults to true");
        remarks.insert(("database_structure.users", "column_uuid"), "# Optional: defaults to \"uuid\"");
        remarks.insert(("database_structure.users", "column_id"), "# Optional: defaults to \"id\"");
        remarks.insert(("database_structure.users", "column_key"), "# Optional: defaults to \"key\"");
        remarks.insert(("database_structure.users", "bin_type_key"), "# Optional: defaults to true");
        remarks.insert(("database_structure.users", "column_uploaded"), "# Optional: defaults to \"uploaded\"");
        remarks.insert(("database_structure.users", "column_downloaded"), "# Optional: defaults to \"downloaded\"");
        remarks.insert(("database_structure.users", "column_completed"), "# Optional: defaults to \"completed\"");
        remarks.insert(("database_structure.users", "column_updated"), "# Optional: defaults to \"updated\"");
        remarks.insert(("database_structure.users", "column_active"), "# Optional: defaults to \"active\"");
        let mut section_remarks: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        section_remarks.insert("sentry_config", "# Optional section: the entire [sentry_config] block can be omitted (defaults to disabled)");
        section_remarks.insert("database_structure.torrents", "# Optional section: omit to use default table/column names for torrents");
        section_remarks.insert("database_structure.whitelist", "# Optional section: omit to use default table/column names for whitelist");
        section_remarks.insert("database_structure.blacklist", "# Optional section: omit to use default table/column names for blacklist");
        section_remarks.insert("database_structure.keys", "# Optional section: omit to use default table/column names for keys");
        section_remarks.insert("database_structure.users", "# Optional section: omit to use default table/column names for users");
        let mut result = String::with_capacity(toml.len() + 4096);
        let mut current_section = String::new();
        for line in toml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
                let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
                current_section = inner.to_string();
                if let Some(remark) = section_remarks.get(inner) {
                    result.push_str(remark);
                    result.push('\n');
                }
                result.push_str(line);
                result.push('\n');
                continue;
            }
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                if !key.is_empty() && !key.starts_with('#')
                    && let Some(remark) = remarks.get(&(current_section.as_str(), key)) {
                    result.push_str(remark);
                    result.push('\n');
                }
            }
            result.push_str(line);
            result.push('\n');
        }
        result
    }
}