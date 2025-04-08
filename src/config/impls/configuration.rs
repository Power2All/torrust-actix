use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread::available_parallelism;
use regex::Regex;
use crate::common::structs::custom_error::CustomError;
use crate::config::enums::configuration_error::ConfigurationError;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
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
use std::env;

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
                prometheus_id: String::from("torrust_actix")
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
                    threads: available_parallelism().unwrap().get() as u64,
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
            )
        }
    }
    
    #[tracing::instrument(level = "debug")]
    pub fn env_overrides(config: &mut Configuration) -> &mut Configuration {
        // Config
        if let Ok(value) = env::var("LOG_LEVEL") { config.log_level = value; }
        if let Ok(value) = env::var("LOG_CONSOLE_INTERVAL") { config.log_console_interval = value.parse::<u64>().unwrap_or(60u64); }
        
        // Tracker config
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
        
        // Sentry config
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
        
        // Database config
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

        // Database Structure Torrents config
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

        // Database Structure Whitelist config
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH") {
            config.database_structure.whitelist.bin_type_infohash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__TABLE_NAME") {
            config.database_structure.whitelist.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH") {
            config.database_structure.whitelist.column_infohash = value;
        }

        // Database Structure Blacklist config
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__BIN_TYPE_INFOHASH") {
            config.database_structure.blacklist.bin_type_infohash = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__TABLE_NAME") {
            config.database_structure.blacklist.table_name = value;
        }
        if let Ok(value) = env::var("DATABASE_STRUCTURE__BLACKLIST__COLUMN_INFOHASH") {
            config.database_structure.blacklist.column_infohash = value;
        }

        // Database Structure Keys config
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

        // Database Structure Users config
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

        // Possible overrides for the API stack
        let mut api_iteration = 0;
        loop {
            match config.api_server.get_mut(api_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("API_{}_ENABLED", api_iteration)) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("API_{}_SSL", api_iteration)) {
                        block.ssl = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                    if let Ok(value) = env::var(format!("API_{}_BIND_ADDRESS", api_iteration)) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("API_{}_REAL_IP", api_iteration)) {
                        block.real_ip = value;
                    }
                    if let Ok(value) = env::var(format!("API_{}_SSL_KEY", api_iteration)) {
                        block.ssl_key = value;
                    }
                    if let Ok(value) = env::var(format!("API_{}_SSL_CERT", api_iteration)) {
                        block.ssl_cert = value;
                    }
                    if let Ok(value) = env::var(format!("API_{}_KEEP_ALIVE", api_iteration)) {
                        block.keep_alive = value.parse::<u64>().unwrap_or(60);
                    }
                    if let Ok(value) = env::var(format!("API_{}_REQUEST_TIMEOUT", api_iteration)) {
                        block.request_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("API_{}_DISCONNECT_TIMEOUT", api_iteration)) {
                        block.disconnect_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("API_{}_MAX_CONNECTIONS", api_iteration)) {
                        block.max_connections = value.parse::<u64>().unwrap_or(25000);
                    }
                    if let Ok(value) = env::var(format!("API_{}_THREADS", api_iteration)) {
                        block.threads = value.parse::<u64>().unwrap_or(available_parallelism().unwrap().get() as u64);
                    }
                    if let Ok(value) = env::var(format!("API_{}_TLS_CONNECTION_RATE", api_iteration)) {
                        block.tls_connection_rate = value.parse::<u64>().unwrap_or(256);
                    }
                }
            }
            api_iteration += 1;
        }

        // Possible overrides for the HTTP stack
        let mut http_iteration = 0;
        loop {
            match config.http_server.get_mut(http_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("HTTP_{}_ENABLED", http_iteration)) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_SSL", http_iteration)) {
                        block.ssl = match value.as_str() { "true" => { true } "false" => { false } _ => { false } };
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_BIND_ADDRESS", http_iteration)) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_REAL_IP", http_iteration)) {
                        block.real_ip = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_SSL_KEY", http_iteration)) {
                        block.ssl_key = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_SSL_CERT", http_iteration)) {
                        block.ssl_cert = value;
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_KEEP_ALIVE", http_iteration)) {
                        block.keep_alive = value.parse::<u64>().unwrap_or(60);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_REQUEST_TIMEOUT", http_iteration)) {
                        block.request_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_DISCONNECT_TIMEOUT", http_iteration)) {
                        block.disconnect_timeout = value.parse::<u64>().unwrap_or(30);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_MAX_CONNECTIONS", http_iteration)) {
                        block.max_connections = value.parse::<u64>().unwrap_or(25000);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_THREADS", http_iteration)) {
                        block.threads = value.parse::<u64>().unwrap_or(available_parallelism().unwrap().get() as u64);
                    }
                    if let Ok(value) = env::var(format!("HTTP_{}_TLS_CONNECTION_RATE", http_iteration)) {
                        block.tls_connection_rate = value.parse::<u64>().unwrap_or(256);
                    }
                }
            }
            http_iteration += 1;
        }

        // Possible overrides for the UDP stack
        let mut udp_iteration = 0;
        loop {
            match config.udp_server.get_mut(udp_iteration) {
                None => {
                    break;
                }
                Some(block) => {
                    if let Ok(value) = env::var(format!("UDP_{}_ENABLED", udp_iteration)) {
                        block.enabled = match value.as_str() { "true" => { true } "false" => { false } _ => { true } };
                    }
                    if let Ok(value) = env::var(format!("UDP_{}_BIND_ADDRESS", udp_iteration)) {
                        block.bind_address = value;
                    }
                    if let Ok(value) = env::var(format!("UDP_{}_THREADS", udp_iteration)) {
                        block.threads = value.parse::<u64>().unwrap_or(available_parallelism().unwrap().get() as u64);
                    }
                }
            }
            udp_iteration += 1;
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
            Err(_) => { eprintln!("[CONFIG SAVE] Unable to save to {}", path); }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn load_from_file(create: bool) -> Result<Configuration, CustomError> {
        let mut config = Configuration::init();
        match Configuration::load_file("config.toml") {
            Ok(c) => { config = c; }
            Err(error) => {
                eprintln!("No config file found or corrupt.");
                eprintln!("[ERROR] {}", error);

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
        // Check Map
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

        // Validation
        for (name, value, regex) in check_map {
            Self::validate_value(name, value, regex);
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn validate_value(name: &str, value: String, regex: String)
    {
        let regex_check = Regex::new(regex.as_str()).unwrap();
        if !regex_check.is_match(value.as_str()){
            panic!("[VALIDATE CONFIG] Error checking {} [:] Name: \"{}\" [:] Regex: \"{}\"", name, value, regex_check);
        }
    }
}