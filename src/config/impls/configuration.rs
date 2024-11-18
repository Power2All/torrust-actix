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

impl Configuration {
    #[tracing::instrument]
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
                total_downloads: 0,
                swagger: false,
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

    #[tracing::instrument]
    pub fn load(data: &[u8]) -> Result<Configuration, toml::de::Error> {
        toml::from_str(&String::from_utf8_lossy(data))
    }

    #[tracing::instrument]
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

    #[tracing::instrument]
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

    #[tracing::instrument]
    pub fn save_from_config(config: Arc<Configuration>, path: &str)
    {
        let config_toml = toml::to_string(&config).unwrap();
        match Self::save_file(path, config_toml) {
            Ok(_) => { eprintln!("[CONFIG SAVE] Config file is saved"); }
            Err(_) => { eprintln!("[CONFIG SAVE] Unable to save to {}", path); }
        }
    }

    #[tracing::instrument]
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

        println!("[VALIDATE] Validating configuration...");
        Self::validate(config.clone());
        Ok(config)
    }

    #[tracing::instrument]
    pub fn validate(config: Configuration) {
        // Check Map
        let check_map = vec![
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

    #[tracing::instrument]
    pub fn validate_value(name: &str, value: String, regex: String)
    {
        let regex_check = Regex::new(regex.as_str()).unwrap();
        if !regex_check.is_match(value.as_str()){
            panic!("[VALIDATE CONFIG] Error checking {} [:] Name: \"{}\" [:] Regex: \"{}\"", name, value, regex_check);
        }
    }
}