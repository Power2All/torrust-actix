use crate::cache::enums::cache_engine::CacheEngine;
use crate::common::structs::custom_error::CustomError;
use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
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
use crate::config::structs::webtorrent_trackers_config::WebTorrentTrackersConfig;
use crate::database::enums::database_drivers::DatabaseDrivers;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread::available_parallelism;

impl Configuration {
    #[tracing::instrument(level = "debug")]
    pub fn init() -> Configuration {
        Configuration {
            log_level: String::from("info"),
            log_console_interval: 60,
            tracker_config: TrackerConfig {
                api_key: String::from("MyApiKey"),
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
                cluster_token: String::from(""),
                cluster_bind_address: String::from("0.0.0.0:8888"),
                cluster_master_address: String::from(""),
                cluster_keep_alive: 60,
                cluster_request_timeout: 15,
                cluster_disconnect_timeout: 15,
                cluster_reconnect_interval: 5,
                cluster_max_connections: 25000,
                cluster_threads: available_parallelism().unwrap().get() as u64,
                cluster_ssl: false,
                cluster_ssl_key: String::from(""),
                cluster_ssl_cert: String::from(""),
                cluster_tls_connection_rate: 256,
            },
            sentry_config: SentryConfig {
                enabled: false,
                dsn: "".to_string(),
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
                    column_completed: String::from("completed")
                },
                whitelist: DatabaseStructureConfigWhitelist {
                    table_name: String::from("whitelist"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                },
                blacklist: DatabaseStructureConfigBlacklist {
                    table_name: String::from("blacklist"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                },
                keys: DatabaseStructureConfigKeys {
                    table_name: String::from("keys"),
                    column_hash: String::from("hash"),
                    bin_type_hash: true,
                    column_timeout: String::from("timeout")
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
                }
            },
            http_server: vec!(
                HttpTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:6969"),
                    real_ip: String::from("X-Real-IP"),
                    keep_alive: 60,
                    request_timeout: 15,
                    disconnect_timeout: 15,
                    max_connections: 25000,
                    threads: available_parallelism().unwrap().get() as u64,
                    ssl: false,
                    ssl_key: String::from(""),
                    ssl_cert: String::from(""),
                    tls_connection_rate: 256
                }
            ),
            udp_server: vec!(
                UdpTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:6969"),
                    udp_threads: 2,
                    worker_threads: available_parallelism().unwrap().get(),
                    receive_buffer_size: 134217728,
                    send_buffer_size: 67108864,
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
                    keep_alive: 60,
                    request_timeout: 30,
                    disconnect_timeout: 30,
                    max_connections: 25000,
                    threads: available_parallelism().unwrap().get() as u64,
                    ssl: false,
                    ssl_key: String::from(""),
                    ssl_cert: String::from(""),
                    tls_connection_rate: 256
                }
            ),
            webtorrent_server: vec!(
                WebTorrentTrackersConfig {
                    enabled: false,
                    bind_address: String::from("0.0.0.0:12100"),
                    keep_alive: 60,
                    request_timeout: 10,
                    disconnect_timeout: 10,
                    max_connections: 100,
                    threads: 4,
                    ssl: false,
                    ssl_key: String::from(""),
                    ssl_cert: String::from(""),
                    tls_connection_rate: 100
                }
            )
        }
    }
    
    #[tracing::instrument(level = "debug")]
    pub fn env_overrides(config: &mut Configuration) -> &mut Configuration {
        if let Ok(value) = env::var("LOG_LEVEL") { config.log_level = value; }
        if let Ok(value) = env::var("LOG_CONSOLE_INTERVAL") { config.log_console_interval = value.parse::<u64>().unwrap_or(60u64); }
        if let Ok(value) = env::var("TRACKER__API_KEY") {
            config.tracker_config.api_key = value
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
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH") {
            config.database_structure.whitelist.bin_type_infohash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__TABLE_NAME") {
            config.database_structure.whitelist.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH") {
            config.database_structure.whitelist.column_infohash = value;
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
                        block.receive_buffer_size = value.parse::<usize>().unwrap_or(134217728);
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_SEND_BUFFER_SIZE")) {
                        block.send_buffer_size = value.parse::<usize>().unwrap_or(67108864);
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_REUSE_ADDRESS")) {
                        block.reuse_address = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("UDP_{udp_iteration}_SIMPLE_PROXY_PROTOCOL")) {
                        block.reuse_address = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                }
            }
            udp_iteration += 1;
        }
        let mut webtorrent_iteration = 0;
        loop {
            match config.webtorrent_server.get_mut(webtorrent_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_ENABLED")) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_SSL")) {
                        block.ssl = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_BIND_ADDRESS")) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_SSL_KEY")) {
                        block.ssl_key = value;
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_SSL_CERT")) {
                        block.ssl_cert = value;
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_KEEP_ALIVE")) {
                        block.keep_alive = value.parse::<u64>().unwrap_or(60);
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_REQUEST_TIMEOUT")) {
                        block.request_timeout = value.parse::<u64>().unwrap_or(10);
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_DISCONNECT_TIMEOUT")) {
                        block.disconnect_timeout = value.parse::<u64>().unwrap_or(10);
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_MAX_CONNECTIONS")) {
                        block.max_connections = value.parse::<u64>().unwrap_or(100);
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_THREADS")) {
                        block.threads = value.parse::<u64>().unwrap_or(4);
                    }
                    if let Ok(value) = env::var(format!("WEBTORRENT_{webtorrent_iteration}_TLS_CONNECTION_RATE")) {
                        block.tls_connection_rate = value.parse::<u64>().unwrap_or(100);
                    }
                }
            }
            webtorrent_iteration += 1;
        }
        config
    }

    #[tracing::instrument(level = "debug")]
    pub fn load(data: &[u8]) -> Result<Configuration, toml::de::Error> {
        toml::from_str(&String::from_utf8_lossy(data))
    }

    #[tracing::instrument(level = "debug")]
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

    #[tracing::instrument(level = "debug")]
    pub fn save_file(path: &str, data: String) -> Result<(), ConfigurationError> {
        match File::create(path) {
            Ok(mut file) => {
                match file.write_all(data.as_ref()) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(ConfigurationError::IOError(e))
                }
            }
            Err(e) => Err(ConfigurationError::IOError(e))
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn save_from_config(config: Arc<Configuration>, path: &str)
    {
        let config_toml = toml::to_string(&config).unwrap();
        match Self::save_file(path, config_toml) {
            Ok(_) => { eprintln!("[CONFIG SAVE] Config file is saved"); }
            Err(_) => { eprintln!("[CONFIG SAVE] Unable to save to {path}"); }
        }
    }

    #[tracing::instrument(level = "debug")]
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
                let config_toml = toml::to_string(&config).unwrap();
                let save_file = Configuration::save_file("config.toml", config_toml);
                return match save_file {
                    Ok(_) => {
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
        };
        Self::env_overrides(&mut config);
        println!("[VALIDATE] Validating configuration...");
        Self::validate(config.clone());
        Ok(config)
    }

    #[tracing::instrument(level = "debug")]
    pub fn validate(config: Configuration) {
        
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
                    &format!("api_server[{}].bind_address", index),
                    &api_server.bind_address,
                );
            }
        }
        for (index, http_server) in config.http_server.iter().enumerate() {
            if http_server.enabled {
                Self::validate_socket_address(
                    &format!("http_server[{}].bind_address", index),
                    &http_server.bind_address,
                );
            }
        }
        for (index, udp_server) in config.udp_server.iter().enumerate() {
            if udp_server.enabled {
                Self::validate_socket_address(
                    &format!("udp_server[{}].bind_address", index),
                    &udp_server.bind_address,
                );
            }
        }
        for (index, webtorrent_server) in config.webtorrent_server.iter().enumerate() {
            if webtorrent_server.enabled {
                Self::validate_socket_address(
                    &format!("webtorrent_server[{}].bind_address", index),
                    &webtorrent_server.bind_address,
                );
            }
        }
        Self::validate_cluster(&config);
        Self::validate_cache(&config);
    }

    #[tracing::instrument(level = "debug")]
    pub fn validate_cache(config: &Configuration) {
        if let Some(ref cache) = config.cache {
            if cache.enabled {
                println!("[VALIDATE] Cache enabled: {}", cache.engine);
                Self::validate_socket_address("cache.address", &cache.address);
                println!("[VALIDATE] Cache prefix: {}", cache.prefix);
                println!("[VALIDATE] Cache TTL: {} seconds", cache.ttl);
            } else {
                println!("[VALIDATE] Cache: disabled");
            }
        } else {
            println!("[VALIDATE] Cache: not configured");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn validate_cluster(config: &Configuration) {
        match config.tracker_config.cluster {
            ClusterMode::standalone => {
                println!("[VALIDATE] Cluster mode: standalone");
            }
            ClusterMode::master => {
                println!("[VALIDATE] Cluster mode: master");
                if config.tracker_config.cluster_token.is_empty() {
                    panic!("[VALIDATE CONFIG] Cluster mode 'master' requires 'cluster_token' to be set for authentication");
                }
                if !config.tracker_config.cluster_ssl {
                    eprintln!("[VALIDATE WARNING] Cluster SSL is disabled - cluster_token will be transmitted in plaintext!");
                }
                if config.tracker_config.cluster_bind_address.is_empty() {
                    panic!("[VALIDATE CONFIG] Cluster mode 'master' requires 'cluster_bind_address' to be set");
                }
                Self::validate_socket_address("cluster_bind_address", &config.tracker_config.cluster_bind_address);
                if config.tracker_config.cluster_ssl {
                    if config.tracker_config.cluster_ssl_key.is_empty() {
                        panic!("[VALIDATE CONFIG] Cluster SSL enabled but 'cluster_ssl_key' is not set");
                    }
                    if config.tracker_config.cluster_ssl_cert.is_empty() {
                        panic!("[VALIDATE CONFIG] Cluster SSL enabled but 'cluster_ssl_cert' is not set");
                    }
                }
                println!("[VALIDATE] Cluster encoding: {:?}", config.tracker_config.cluster_encoding);
                println!("[VALIDATE] Cluster bind address: {}", config.tracker_config.cluster_bind_address);
                println!("[VALIDATE] Cluster SSL: {}", if config.tracker_config.cluster_ssl { "wss" } else { "ws" });
            }
            ClusterMode::slave => {
                println!("[VALIDATE] Cluster mode: slave");
                if config.tracker_config.cluster_token.is_empty() {
                    panic!("[VALIDATE CONFIG] Cluster mode 'slave' requires 'cluster_token' to be set for authentication");
                }
                if config.tracker_config.cluster_master_address.is_empty() {
                    panic!("[VALIDATE CONFIG] Cluster mode 'slave' requires 'cluster_master_address' to be set");
                }
                Self::validate_socket_address("cluster_master_address", &config.tracker_config.cluster_master_address);
                println!("[VALIDATE] Cluster master address: {}", config.tracker_config.cluster_master_address);
                println!("[VALIDATE] Cluster SSL: {}", if config.tracker_config.cluster_ssl { "wss" } else { "ws" });
            }
        }
    }
    
    #[tracing::instrument(level = "debug")]
    pub fn validate_socket_address(field_name: &str, address: &str) {
        use std::net::SocketAddr;
        match address.parse::<SocketAddr>() {
            Ok(addr) => {
                println!("[VALIDATE] {} is valid: {}", field_name, addr);
            }
            Err(e) => {
                panic!(
                    "[VALIDATE CONFIG] '{}' has invalid format: '{}'. \
                    Expected IPv4 format '1.2.3.4:8888' or IPv6 format '[2a00:1768:1001:0026::0183]:80'. \
                    Error: {}",
                    field_name, address, e
                );
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn validate_value(name: &str, value: String, regex: String)
    {
        let regex_check = Regex::new(regex.as_str()).unwrap();
        if !regex_check.is_match(value.as_str()){
            panic!("[VALIDATE CONFIG] Error checking {name} [:] Name: \"{value}\" [:] Regex: \"{regex_check}\"");
        }
    }
}