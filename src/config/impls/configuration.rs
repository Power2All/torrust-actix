use crate::cache::enums::cache_engine::CacheEngine;
use crate::common::structs::custom_error::CustomError;
use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
use crate::config::enums::compression_algorithm::CompressionAlgorithm;
use crate::config::enums::configuration_error::ConfigurationError;
use crate::config::enums::udp_receive_method::UdpReceiveMethod;
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
use std::env;
use std::io::Write;
use std::thread::available_parallelism;

/// Overrides `$field` from the environment variable `$name` when it is set. A string field takes
/// the value as-is; a `bool` or `num` field keeps its loaded value (with a warning) when the value
/// does not parse, so a typo cannot silently reset a setting such as `max_torrents` (`0` =
/// unlimited) to its default; an `opt_bool` field is left unset. The warnings go to stderr: this
/// runs while the configuration is read, before the logger exists.
macro_rules! env_override {
    ($name:expr, $field:expr) => {
        if let Ok(value) = env::var($name) { $field = value; }
    };
    ($name:expr, $field:expr, bool) => {
        if let Ok(value) = env::var($name) { $field = parse_env_bool($name, &value, $field); }
    };
    ($name:expr, $field:expr, num) => {
        if let Ok(value) = env::var($name) { $field = parse_env_num($name, &value, $field); }
    };
    ($name:expr, $field:expr, opt_bool) => {
        if let Ok(value) = env::var($name) { $field = parse_env_optional_bool($name, &value); }
    };
}

impl Configuration {
    /// Builds the built-in default configuration, including a freshly generated secure API key.
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
                max_peers_per_torrent: 10_000,
                max_rtc_pending_answers: 32,
                max_torrents: 0,
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
                tls_connection_rate: 256,
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
                chunk_size: 1000,
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
                    trusted_proxy_ips: Vec::new(),
                    keep_alive: 60,
                    request_timeout: 15,
                    disconnect_timeout: 15,
                    max_connections: 25000,
                    threads: available_parallelism().unwrap().get() as u64,
                    ssl: false,
                    ssl_key: String::new(),
                    ssl_cert: String::new(),
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
                    proxy_addresses: Vec::new(),
                    receive_method: UdpReceiveMethod::recvmmsg,
                    parse_queue_size: crate::config::config::default_parse_queue_size(),
                }
            ),
            api_server: vec!(
                ApiTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:8080"),
                    real_ip: String::from("X-Real-IP"),
                    trusted_proxies: false,
                    trusted_proxy_ips: Vec::new(),
                    keep_alive: 60,
                    request_timeout: 30,
                    disconnect_timeout: 30,
                    max_connections: 25000,
                    threads: available_parallelism().unwrap().get() as u64,
                    ssl: false,
                    ssl_key: String::new(),
                    ssl_cert: String::new(),
                }
            ),
        }
    }
    /// Applies `SECTION__FIELD`-style environment variable overrides on top of the loaded
    /// configuration (for example `TRACKER__API_KEY` or `DATABASE__CHUNK_SIZE`).
    pub fn env_overrides(config: &mut Configuration) -> &mut Configuration {
        env_override!("LOG_LEVEL", config.log_level);
        env_override!("LOG_CONSOLE_INTERVAL", config.log_console_interval, num);
        env_override!("TRACKER__API_KEY", config.tracker_config.api_key);
        env_override!("TRACKER__WHITELIST_ENABLED", config.tracker_config.whitelist_enabled, bool);
        env_override!("TRACKER__BLACKLIST_ENABLED", config.tracker_config.blacklist_enabled, bool);
        env_override!("TRACKER__KEYS_ENABLED", config.tracker_config.keys_enabled, bool);
        env_override!("TRACKER__USERS_ENABLED", config.tracker_config.users_enabled, bool);
        env_override!("TRACKER__SWAGGER", config.tracker_config.swagger, bool);
        env_override!("TRACKER__KEYS_CLEANUP_INTERVAL", config.tracker_config.keys_cleanup_interval, num);
        env_override!("TRACKER__REQUEST_INTERVAL", config.tracker_config.request_interval, num);
        env_override!("TRACKER__REQUEST_INTERVAL_MINIMUM", config.tracker_config.request_interval_minimum, num);
        env_override!("TRACKER__PEERS_TIMEOUT", config.tracker_config.peers_timeout, num);
        env_override!("TRACKER__PEERS_CLEANUP_INTERVAL", config.tracker_config.peers_cleanup_interval, num);
        env_override!("TRACKER__PEERS_CLEANUP_THREADS", config.tracker_config.peers_cleanup_threads, num);
        env_override!("TRACKER__MAX_PEERS_PER_TORRENT", config.tracker_config.max_peers_per_torrent, num);
        env_override!("TRACKER__MAX_RTC_PENDING_ANSWERS", config.tracker_config.max_rtc_pending_answers, num);
        env_override!("TRACKER__MAX_TORRENTS", config.tracker_config.max_torrents, num);
        env_override!("TRACKER__PROMETHEUS_ID", config.tracker_config.prometheus_id);
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
        env_override!("TRACKER__CLUSTER_TOKEN", config.tracker_config.cluster_token);
        env_override!("TRACKER__CLUSTER_BIND_ADDRESS", config.tracker_config.cluster_bind_address);
        env_override!("TRACKER__CLUSTER_MASTER_ADDRESS", config.tracker_config.cluster_master_address);
        env_override!("TRACKER__CLUSTER_KEEP_ALIVE", config.tracker_config.cluster_keep_alive, num);
        env_override!("TRACKER__CLUSTER_REQUEST_TIMEOUT", config.tracker_config.cluster_request_timeout, num);
        env_override!("TRACKER__CLUSTER_DISCONNECT_TIMEOUT", config.tracker_config.cluster_disconnect_timeout, num);
        env_override!("TRACKER__CLUSTER_RECONNECT_INTERVAL", config.tracker_config.cluster_reconnect_interval, num);
        env_override!("TRACKER__CLUSTER_MAX_CONNECTIONS", config.tracker_config.cluster_max_connections, num);
        env_override!("TRACKER__CLUSTER_THREADS", config.tracker_config.cluster_threads, num);
        env_override!("TRACKER__CLUSTER_SSL", config.tracker_config.cluster_ssl, bool);
        env_override!("TRACKER__CLUSTER_SSL_KEY", config.tracker_config.cluster_ssl_key);
        env_override!("TRACKER__CLUSTER_SSL_CERT", config.tracker_config.cluster_ssl_cert);
        env_override!("TRACKER__TLS_CONNECTION_RATE", config.tracker_config.tls_connection_rate, num);
        env_override!("TRACKER__RTC_INTERVAL", config.tracker_config.rtc_interval, num);
        env_override!("TRACKER__RTC_PEERS_TIMEOUT", config.tracker_config.rtc_peers_timeout, num);
        env_override!("TRACKER__TOTAL_DOWNLOADS", config.tracker_config.total_downloads, num);
        env_override!("TRACKER__RTC_COMPRESSION_ENABLED", config.tracker_config.rtc_compression_enabled, bool);
        if let Ok(value) = env::var("TRACKER__RTC_COMPRESSION_ALGORITHM") {
            config.tracker_config.rtc_compression_algorithm = match value.to_lowercase().as_str() {
                "zstd" => CompressionAlgorithm::Zstd,
                _ => CompressionAlgorithm::Lz4,
            };
        }
        env_override!("TRACKER__RTC_COMPRESSION_LEVEL", config.tracker_config.rtc_compression_level, num);
        env_override!("SENTRY__ENABLED", config.sentry_config.enabled, bool);
        env_override!("SENTRY__DEBUG", config.sentry_config.debug, bool);
        env_override!("SENTRY__ATTACH_STACKTRACE", config.sentry_config.attach_stacktrace, bool);
        env_override!("SENTRY__SEND_DEFAULT_PII", config.sentry_config.send_default_pii, bool);
        env_override!("SENTRY__DSN", config.sentry_config.dsn);
        env_override!("SENTRY__MAX_BREADCRUMBS", config.sentry_config.max_breadcrumbs, num);
        env_override!("SENTRY__SAMPLE_RATE", config.sentry_config.sample_rate, num);
        env_override!("SENTRY__TRACES_SAMPLE_RATE", config.sentry_config.traces_sample_rate, num);
        env_override!("DATABASE__PERSISTENT", config.database.persistent, bool);
        env_override!("DATABASE__INSERT_VACANT", config.database.insert_vacant, bool);
        env_override!("DATABASE__REMOVE_ACTION", config.database.remove_action, bool);
        env_override!("DATABASE__UPDATE_COMPLETED", config.database.update_completed, bool);
        env_override!("DATABASE__UPDATE_PEERS", config.database.update_peers, bool);
        env_override!("DATABASE__PATH", config.database.path);
        if let Ok(value) = env::var("DATABASE__ENGINE") {
            config.database.engine = match value.as_str() {
                "sqlite3" => { DatabaseDrivers::sqlite3 }
                "mysql" => { DatabaseDrivers::mysql }
                "pgsql" => { DatabaseDrivers::pgsql }
                _ => { DatabaseDrivers::sqlite3 }
            };
        }
        env_override!("DATABASE__PERSISTENT_INTERVAL", config.database.persistent_interval, num);
        env_override!("DATABASE__CHUNK_SIZE", config.database.chunk_size, num);
        if let Ok(value) = env::var("CACHE__ENABLED") {
            let enabled = parse_env_bool("CACHE__ENABLED", &value, false);
            // Only `true` creates a missing [cache] section; the other cache variables always do.
            if enabled || config.cache.is_some() {
                config.cache.get_or_insert_with(CacheConfig::default).enabled = enabled;
            }
        }
        if let Ok(value) = env::var("CACHE__ENGINE") {
            config.cache.get_or_insert_with(CacheConfig::default).engine = match value.as_str() {
                "redis" => CacheEngine::redis,
                "memcache" => CacheEngine::memcache,
                _ => CacheEngine::redis
            };
        }
        env_override!("CACHE__ADDRESS", config.cache.get_or_insert_with(CacheConfig::default).address);
        env_override!("CACHE__PREFIX", config.cache.get_or_insert_with(CacheConfig::default).prefix);
        env_override!("CACHE__TTL", config.cache.get_or_insert_with(CacheConfig::default).ttl, num);
        env_override!("CACHE__SPLIT_PEERS", config.cache.get_or_insert_with(CacheConfig::default).split_peers, bool);
        env_override!("DATABASE_STRUCTURE__TORRENTS__BIN_TYPE_INFOHASH", config.database_structure.torrents.bin_type_infohash, bool);
        env_override!("DATABASE_STRUCTURE__TORRENTS__TABLE_NAME", config.database_structure.torrents.table_name);
        env_override!("DATABASE_STRUCTURE__TORRENTS__COLUMN_INFOHASH", config.database_structure.torrents.column_infohash);
        env_override!("DATABASE_STRUCTURE__TORRENTS__COLUMN_SEEDS", config.database_structure.torrents.column_seeds);
        env_override!("DATABASE_STRUCTURE__TORRENTS__COLUMN_PEERS", config.database_structure.torrents.column_peers);
        env_override!("DATABASE_STRUCTURE__TORRENTS__COLUMN_COMPLETED", config.database_structure.torrents.column_completed);
        env_override!("DATABASE_STRUCTURE__TORRENTS__PERSISTENT", config.database_structure.torrents.persistent, opt_bool);
        env_override!("DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH", config.database_structure.whitelist.bin_type_infohash, bool);
        env_override!("DATABASE_STRUCTURE__WHITELIST__TABLE_NAME", config.database_structure.whitelist.table_name);
        env_override!("DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH", config.database_structure.whitelist.column_infohash);
        env_override!("DATABASE_STRUCTURE__WHITELIST__PERSISTENT", config.database_structure.whitelist.persistent, opt_bool);
        env_override!("DATABASE_STRUCTURE__BLACKLIST__BIN_TYPE_INFOHASH", config.database_structure.blacklist.bin_type_infohash, bool);
        env_override!("DATABASE_STRUCTURE__BLACKLIST__TABLE_NAME", config.database_structure.blacklist.table_name);
        env_override!("DATABASE_STRUCTURE__BLACKLIST__COLUMN_INFOHASH", config.database_structure.blacklist.column_infohash);
        env_override!("DATABASE_STRUCTURE__BLACKLIST__PERSISTENT", config.database_structure.blacklist.persistent, opt_bool);
        env_override!("DATABASE_STRUCTURE__KEYS__BIN_TYPE_HASH", config.database_structure.keys.bin_type_hash, bool);
        env_override!("DATABASE_STRUCTURE__KEYS__TABLE_NAME", config.database_structure.keys.table_name);
        env_override!("DATABASE_STRUCTURE__KEYS__COLUMN_HASH", config.database_structure.keys.column_hash);
        env_override!("DATABASE_STRUCTURE__KEYS__COLUMN_TIMEOUT", config.database_structure.keys.column_timeout);
        env_override!("DATABASE_STRUCTURE__KEYS__PERSISTENT", config.database_structure.keys.persistent, opt_bool);
        env_override!("DATABASE_STRUCTURE__USERS__ID_UUID", config.database_structure.users.id_uuid, bool);
        env_override!("DATABASE_STRUCTURE__USERS__BIN_TYPE_KEY", config.database_structure.users.bin_type_key, bool);
        env_override!("DATABASE_STRUCTURE__USERS__TABLE_NAME", config.database_structure.users.table_name);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_UUID", config.database_structure.users.column_uuid);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_ID", config.database_structure.users.column_id);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_ACTIVE", config.database_structure.users.column_active);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_KEY", config.database_structure.users.column_key);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_UPLOADED", config.database_structure.users.column_uploaded);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_DOWNLOADED", config.database_structure.users.column_downloaded);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_COMPLETED", config.database_structure.users.column_completed);
        env_override!("DATABASE_STRUCTURE__USERS__COLUMN_UPDATED", config.database_structure.users.column_updated);
        env_override!("DATABASE_STRUCTURE__USERS__PERSISTENT", config.database_structure.users.persistent, opt_bool);
        for (api_iteration, block) in config.api_server.iter_mut().enumerate() {
            env_override!(format!("API_{api_iteration}_ENABLED"), block.enabled, bool);
            env_override!(format!("API_{api_iteration}_SSL"), block.ssl, bool);
            env_override!(format!("API_{api_iteration}_BIND_ADDRESS"), block.bind_address);
            env_override!(format!("API_{api_iteration}_REAL_IP"), block.real_ip);
            env_override!(format!("API_{api_iteration}_SSL_KEY"), block.ssl_key);
            env_override!(format!("API_{api_iteration}_SSL_CERT"), block.ssl_cert);
            env_override!(format!("API_{api_iteration}_KEEP_ALIVE"), block.keep_alive, num);
            env_override!(format!("API_{api_iteration}_REQUEST_TIMEOUT"), block.request_timeout, num);
            env_override!(format!("API_{api_iteration}_DISCONNECT_TIMEOUT"), block.disconnect_timeout, num);
            env_override!(format!("API_{api_iteration}_MAX_CONNECTIONS"), block.max_connections, num);
            env_override!(format!("API_{api_iteration}_THREADS"), block.threads, num);
        }
        for (http_iteration, block) in config.http_server.iter_mut().enumerate() {
            env_override!(format!("HTTP_{http_iteration}_ENABLED"), block.enabled, bool);
            env_override!(format!("HTTP_{http_iteration}_SSL"), block.ssl, bool);
            env_override!(format!("HTTP_{http_iteration}_BIND_ADDRESS"), block.bind_address);
            env_override!(format!("HTTP_{http_iteration}_REAL_IP"), block.real_ip);
            env_override!(format!("HTTP_{http_iteration}_SSL_KEY"), block.ssl_key);
            env_override!(format!("HTTP_{http_iteration}_SSL_CERT"), block.ssl_cert);
            env_override!(format!("HTTP_{http_iteration}_KEEP_ALIVE"), block.keep_alive, num);
            env_override!(format!("HTTP_{http_iteration}_REQUEST_TIMEOUT"), block.request_timeout, num);
            env_override!(format!("HTTP_{http_iteration}_DISCONNECT_TIMEOUT"), block.disconnect_timeout, num);
            env_override!(format!("HTTP_{http_iteration}_MAX_CONNECTIONS"), block.max_connections, num);
            env_override!(format!("HTTP_{http_iteration}_THREADS"), block.threads, num);
            env_override!(format!("HTTP_{http_iteration}_RTCTORRENT"), block.rtctorrent, bool);
        }
        for (udp_iteration, block) in config.udp_server.iter_mut().enumerate() {
            env_override!(format!("UDP_{udp_iteration}_ENABLED"), block.enabled, bool);
            env_override!(format!("UDP_{udp_iteration}_BIND_ADDRESS"), block.bind_address);
            env_override!(format!("UDP_{udp_iteration}_UDP_THREADS"), block.udp_threads, num);
            env_override!(format!("UDP_{udp_iteration}_WORKER_THREADS"), block.worker_threads, num);
            env_override!(format!("UDP_{udp_iteration}_RECEIVE_BUFFER_SIZE"), block.receive_buffer_size, num);
            env_override!(format!("UDP_{udp_iteration}_SEND_BUFFER_SIZE"), block.send_buffer_size, num);
            env_override!(format!("UDP_{udp_iteration}_REUSE_ADDRESS"), block.reuse_address, bool);
            env_override!(format!("UDP_{udp_iteration}_PARSE_QUEUE_SIZE"), block.parse_queue_size, num);
            env_override!(format!("UDP_{udp_iteration}_USE_PAYLOAD_IP"), block.use_payload_ip, bool);
            env_override!(format!("UDP_{udp_iteration}_SIMPLE_PROXY_PROTOCOL"), block.simple_proxy_protocol, bool);
            if let Ok(value) = env::var(format!("UDP_{udp_iteration}_RECEIVE_METHOD")) {
                block.receive_method = match value.to_lowercase().as_str() {
                    "io_uring" => UdpReceiveMethod::io_uring,
                    "rio" => UdpReceiveMethod::rio,
                    _ => UdpReceiveMethod::recvmmsg,
                };
            }
        }
        config
    }

    /// Writes the given TOML string to `path`.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigurationError::IOError`] when the file cannot be created or written.
    pub fn save_file(path: &str, data: String) -> Result<(), ConfigurationError> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        // Owner-only: the file holds the API key, the cluster token and the database DSN, which
        // the umask's usual 0644 left readable by every local user. Applies when it is created.
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        match options.open(path) {
            Ok(mut file) => {
                match file.write_all(data.as_ref()) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(ConfigurationError::IOError(e))
                }
            }
            Err(e) => Err(ConfigurationError::IOError(e))
        }
    }

    /// Parses a configuration from raw TOML bytes.
    ///
    /// Invalid UTF-8 sequences are replaced (lossy) before parsing rather than rejected.
    ///
    /// # Errors
    ///
    /// Returns the TOML deserialisation error when the content is invalid.
    pub fn load(data: &[u8]) -> Result<Configuration, toml::de::Error> {
        toml::from_str(&String::from_utf8_lossy(data))
    }

    /// Reads and parses a TOML configuration file.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigurationError`] when the file cannot be read or parsed.
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

    /// Loads `config.toml` from the working directory, applies environment overrides and validates it.
    ///
    /// When the file is missing and `create` is true, an annotated default configuration is written
    /// and an error is returned so the operator can review it before the first start.
    ///
    /// # Errors
    ///
    /// Returns a [`CustomError`] when the file is missing/corrupt or was just created.
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

    /// Validates the complete configuration (API key strength, identifier patterns, tracker,
    /// Sentry, cluster, cache and compression sections).
    ///
    /// # Panics
    ///
    /// Panics with a descriptive message on the first invalid value; called once at startup.
    pub fn validate(config: Configuration) {
        if validate_api_key_strength(&config.tracker_config.api_key) {
            println!("[VALIDATE] API key strength: OK");
        } else {
            eprintln!("[SECURITY WARNING] API key is weak! Please use a stronger API key.");
            eprintln!("[SECURITY WARNING] Generate a secure key with: 'head -c 32 /dev/urandom | base64'");
        }
        let prometheus_id = &config.tracker_config.prometheus_id;
        assert!(
            !prometheus_id.is_empty() && prometheus_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "[VALIDATE CONFIG] Error checking [TRACKER_CONFIG] prometheus_id [:] Name: \"{prometheus_id}\" [:] expected only letters, digits or underscores"
        );
        let db = &config.database_structure;
        let check_map = [
            ("[DB: torrents]", &db.torrents.table_name),
            ("[DB: torrents] Column: infohash", &db.torrents.column_infohash),
            ("[DB: torrents] Column: seeds", &db.torrents.column_seeds),
            ("[DB: torrents] Column: peers", &db.torrents.column_peers),
            ("[DB: torrents] Column: completed", &db.torrents.column_completed),
            ("[DB: whitelist]", &db.whitelist.table_name),
            ("[DB: whitelist] Column: infohash", &db.whitelist.column_infohash),
            ("[DB: blacklist]", &db.blacklist.table_name),
            ("[DB: blacklist] Column: infohash", &db.blacklist.column_infohash),
            ("[DB: keys]", &db.keys.table_name),
            ("[DB: keys] Column: hash", &db.keys.column_hash),
            ("[DB: keys] Column: timeout", &db.keys.column_timeout),
            ("[DB: users]", &db.users.table_name),
            ("[DB: users] Column: id", &db.users.column_id),
            ("[DB: users] Column: uuid", &db.users.column_uuid),
            ("[DB: users] Column: key", &db.users.column_key),
            ("[DB: users] Column: uploaded", &db.users.column_uploaded),
            ("[DB: users] Column: downloaded", &db.users.column_downloaded),
            ("[DB: users] Column: completed", &db.users.column_completed),
            ("[DB: users] Column: active", &db.users.column_active),
            ("[DB: users] Column: updated", &db.users.column_updated),
        ];
        for (name, value) in check_map {
            Self::validate_value(name, value);
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
                if api_server.trusted_proxies && api_server.trusted_proxy_ips.is_empty() {
                    eprintln!("[SECURITY WARNING] api_server[{index}] trusted_proxies=true with an empty trusted_proxy_ips list: the '{}' header is honoured from any source, so a client reaching this listener directly can spoof its IP. Set trusted_proxy_ips to your proxy addresses.", api_server.real_ip);
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
                if http_server.trusted_proxies && http_server.trusted_proxy_ips.is_empty() {
                    eprintln!("[SECURITY WARNING] http_server[{index}] trusted_proxies=true with an empty trusted_proxy_ips list: the '{}' header is honoured from any source, so a client reaching this listener directly can spoof its IP. Set trusted_proxy_ips to your proxy addresses.", http_server.real_ip);
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
                // `ArrayQueue::new(0)` panics, and a queue shallower than one receive batch
                // drops packets the backend has already read off the socket.
                assert!(udp_server.parse_queue_size >= 1024, "[VALIDATE CONFIG] udp_server[{index}] parse_queue_size must be >= 1024");
                if udp_server.simple_proxy_protocol && udp_server.proxy_addresses.is_empty() {
                    eprintln!("[SECURITY WARNING] udp_server[{index}] simple_proxy_protocol=true with an empty proxy_addresses list: the SPP header is trusted from any sender, so anyone can choose the client address the tracker records. Set proxy_addresses to your load balancer addresses.");
                }
                if udp_server.use_payload_ip && udp_server.proxy_addresses.is_empty() {
                    eprintln!("[SECURITY WARNING] udp_server[{index}] use_payload_ip=true with an empty proxy_addresses list: the announce IP field is honoured from any sender, so a client can have its peer entry point at a third party. Set proxy_addresses to your load balancer addresses.");
                }
            }
        }
        Self::validate_tracker(&config);
        Self::validate_sentry(&config);
        Self::validate_cluster(&config);
        Self::validate_cache(&config);
        Self::validate_compression(&config);
    }

    /// Validates tracker timing/thread settings (intervals, timeouts, thread counts).
    ///
    /// # Panics
    ///
    /// Panics when a value is zero or `request_interval_minimum` exceeds `request_interval`.
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
        if tc.max_peers_per_torrent == 0 {
            eprintln!("[SECURITY WARNING] max_peers_per_torrent=0 disables the per-torrent peer limit: a client announcing with made-up peer ids can grow memory until peers_timeout expires them.");
        } else {
            println!("[VALIDATE] max_peers_per_torrent: {} per peer map", tc.max_peers_per_torrent);
        }
        assert!(tc.max_rtc_pending_answers > 0, "[VALIDATE CONFIG] max_rtc_pending_answers must be > 0");
        if tc.max_torrents == 0 {
            if !tc.whitelist_enabled {
                eprintln!("[SECURITY WARNING] max_torrents=0 with whitelist_enabled=false: info_hashes are client-chosen, so nothing bounds how many swarms an announce flood can create. Set max_torrents to what this host can hold.");
            }
        } else {
            println!("[VALIDATE] max_torrents: {}", tc.max_torrents);
        }
        println!("[VALIDATE] rtc_interval: {}s, rtc_peers_timeout: {}s", tc.rtc_interval, tc.rtc_peers_timeout);
    }

    /// Validates the Sentry section.
    ///
    /// # Panics
    ///
    /// Panics when Sentry is enabled without a DSN.
    pub fn validate_sentry(config: &Configuration) {
        let sc = &config.sentry_config;
        if sc.enabled {
            assert!(!sc.dsn.is_empty(), "[VALIDATE CONFIG] sentry.enabled=true but sentry.dsn is not set");
            // sentry's sample_rate/traces_sample_rate builders panic outside [0.0, 1.0]; fail here
            // with a config-shaped message instead of aborting inside sentry::init.
            assert!((0.0..=1.0).contains(&sc.sample_rate), "[VALIDATE CONFIG] sentry.sample_rate must be between 0.0 and 1.0, got {}", sc.sample_rate);
            assert!((0.0..=1.0).contains(&sc.traces_sample_rate), "[VALIDATE CONFIG] sentry.traces_sample_rate must be between 0.0 and 1.0, got {}", sc.traces_sample_rate);
            println!("[VALIDATE] Sentry enabled: dsn configured, sample_rate={}, traces_sample_rate={}", sc.sample_rate, sc.traces_sample_rate);
        } else {
            println!("[VALIDATE] Sentry: disabled");
        }
    }

    /// Validates the optional cache section (engine, address format, TTL) and logs the outcome.
    pub fn validate_cache(config: &Configuration) {
        if let Some(ref cache) = config.cache {
            if cache.enabled {
                println!("[VALIDATE] Cache enabled: {}", cache.engine);
                Self::validate_cache_address(&cache.engine.connection_url(&cache.address));
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

    /// Validates the RTC compression settings.
    ///
    /// # Panics
    ///
    /// Panics when the zstd level is outside 1..=22.
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

    /// Validates the cluster section for the selected mode (token, addresses, SSL material).
    ///
    /// # Panics
    ///
    /// Panics when a required cluster setting is missing or malformed.
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

    /// Checks `cache.address` (as a URL) names a host and a numeric port. Credentials, a host name
    /// and a database path are all allowed: it used to have to be a bare IP:port, which ruled out
    /// Redis AUTH and Docker service names, so the Redis the website reads had to run open.
    ///
    /// # Panics
    ///
    /// Panics with a descriptive message when no host and port can be found.
    pub fn validate_cache_address(url: &str) {
        let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
        let host_port = rest.split('/').next().unwrap_or_default();
        let host_port = host_port.rsplit_once('@').map_or(host_port, |(_, host_port)| host_port);
        let valid = host_port.rsplit_once(':').is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok());
        assert!(
            valid,
            "[VALIDATE CONFIG] 'cache.address' has invalid format: '{}'. Expected 'host:port', \
            optionally with credentials ('user:password@host:port') or as a URL ('redis://:password@host:6379/0')",
            crate::database::database::redact_dsn(url)
        );
        println!("[VALIDATE] cache.address is valid: {}", crate::database::database::redact_dsn(url));
    }

    /// Validates that a string parses as an IPv4/IPv6 `host:port` socket address.
    ///
    /// # Panics
    ///
    /// Panics with a descriptive message when the address is malformed.
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

    /// Asserts that `value` is a valid database identifier: first character a lowercase
    /// letter or underscore, the rest lowercase letters, digits or underscores, at most
    /// 31 characters (equivalent to `^[a-z_][a-z0-9_]{0,30}$`).
    ///
    /// # Panics
    ///
    /// Panics when the value does not match.
    pub fn validate_value(name: &str, value: &str)
    {
        let mut bytes = value.bytes();
        let valid = value.len() <= 31
            && bytes.next().is_some_and(|b| b.is_ascii_lowercase() || b == b'_')
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        assert!(valid, "[VALIDATE CONFIG] Error checking {name} [:] Name: \"{value}\" [:] expected pattern ^[a-z_][a-z0-9_]{{0,30}}$");
    }

    /// Serialises the configuration to TOML with explanatory comments above each setting,
    /// as written by `--create-config`.
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
        remarks.insert(("tracker_config", "max_peers_per_torrent"), "# Optional: defaults to 10000 -- max peers per torrent in EACH of its 6 peer maps, so up to 6x this per torrent (0 = unlimited, not recommended)");
        remarks.insert(("tracker_config", "max_rtc_pending_answers"), "# Optional: defaults to 32 -- max SDP answers queued for one RtcTorrent peer");
        remarks.insert(("tracker_config", "max_torrents"), "# Optional: defaults to 0 (unlimited) -- max distinct torrents held in memory; new info_hashes are refused above it, known ones keep working");
        remarks.insert(("tracker_config", "swagger"), "# Optional: defaults to false -- expose Swagger UI at <api>/swagger-ui/ (the UI assets are served without a token; the spec it loads is not)");
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
        remarks.insert(("tracker_config", "tls_connection_rate"), "# Optional: defaults to 256 -- max concurrent TLS handshakes per worker thread, process-wide (HTTP, API and cluster)");
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
        remarks.insert(("http_server", "trusted_proxy_ips"), "# Optional: defaults to [] -- source IPs allowed to set real_ip; empty trusts any sender, which allows IP spoofing");
        remarks.insert(("api_server", "trusted_proxy_ips"), "# Optional: defaults to [] -- source IPs allowed to set real_ip; empty trusts any sender, which allows IP spoofing");
        remarks.insert(("udp_server", "proxy_addresses"), "# Optional: defaults to [] -- source IPs allowed to send a Simple Proxy Protocol header; empty trusts any sender");
        remarks.insert(("udp_server", "parse_queue_size"), "# Optional: defaults to 65536 -- slots between the receive backend and the parse workers, preallocated at 312 bytes each (~20 MB)");
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
            if trimmed.starts_with('[') {
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

fn parse_env_bool(name: impl std::fmt::Display, value: &str, default: bool) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        "false" | "0" | "no" | "off" => false,
        _ => {
            eprintln!("[CONFIG] Invalid boolean for {name}={value:?}, keeping {default}");
            default
        }
    }
}

fn parse_env_optional_bool(name: impl std::fmt::Display, value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => {
            eprintln!("[CONFIG] Invalid boolean for {name}={value:?}, leaving unset");
            None
        }
    }
}

fn parse_env_num<T>(name: impl std::fmt::Display, value: &str, default: T) -> T
where
    T: std::str::FromStr + std::fmt::Display + Copy,
{
    match value.parse::<T>() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("[CONFIG] Invalid numeric value for {name}={value:?}, keeping {default}");
            default
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::{parse_env_bool, parse_env_num};

    #[test]
    fn env_override_applies_valid_values_and_keeps_the_loaded_one_on_bad_input() {
        // SAFETY: names unique to this test, and no other test reads them.
        unsafe {
            env::set_var("TORRUST_TEST_ENV_OVERRIDE_BAD_NUM", "4x2");
            env::set_var("TORRUST_TEST_ENV_OVERRIDE_NUM", "7");
            env::set_var("TORRUST_TEST_ENV_OVERRIDE_BOOL", "off");
        }
        let (mut bad, mut good, mut flag) = (42u64, 0u64, true);
        env_override!("TORRUST_TEST_ENV_OVERRIDE_BAD_NUM", bad, num);
        env_override!("TORRUST_TEST_ENV_OVERRIDE_NUM", good, num);
        env_override!("TORRUST_TEST_ENV_OVERRIDE_BOOL", flag, bool);
        assert_eq!((bad, good, flag), (42, 7, false));
    }
}
