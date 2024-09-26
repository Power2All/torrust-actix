use std::fs::File;
use std::io::Write;
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
use crate::config::structs::tracker_config::TrackerConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;
use crate::database::enums::database_drivers::DatabaseDrivers;

impl Configuration {
    pub fn init() -> Configuration {
        Configuration {
            log_level: String::from("info"),
            log_console_interval: Some(60),
            tracker_config: Some(TrackerConfig {
                api_key: Some(String::from("MyApiKey")),
                whitelist_enabled: Some(false),
                blacklist_enabled: Some(false),
                keys_enabled: Some(false),
                keys_cleanup_interval: Some(60),
                users_enabled: Some(false),
                request_interval: Some(1800),
                request_interval_minimum: Some(1800),
                peers_timeout: Some(2700),
                peers_cleanup_interval: Some(900),
                total_downloads: 0
            }),
            database: Some(DatabaseConfig {
                engine: Some(DatabaseDrivers::sqlite3),
                path: Some(String::from("sqlite://data.db")),
                persistent: false,
                persistent_interval: Some(60),
                insert_vacant: false,
                update_completed: true,
                update_peers: false,
            }),
            database_structure: Some(DatabaseStructureConfig {
                torrents: Some(DatabaseStructureConfigTorrents {
                    database_name: String::from("torrents"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                    column_seeds: String::from("seeds"),
                    column_peers: String::from("peers"),
                    column_completed: String::from("completed")
                }),
                whitelist: Some(DatabaseStructureConfigWhitelist {
                    database_name: String::from("whitelist"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                }),
                blacklist: Some(DatabaseStructureConfigBlacklist {
                    database_name: String::from("blacklist"),
                    column_infohash: String::from("infohash"),
                    bin_type_infohash: true,
                }),
                keys: Some(DatabaseStructureConfigKeys {
                    database_name: String::from("keys"),
                    column_hash: String::from("hash"),
                    bin_type_hash: true,
                    column_timeout: String::from("timeout")
                }),
                users: Some(DatabaseStructureConfigUsers {
                    database_name: String::from("users"),
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
                })
            }),
            http_server: vec!(
                HttpTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:6969"),
                    real_ip: Some(String::from("X-Real-IP")),
                    keep_alive: Some(60),
                    request_timeout: Some(15),
                    disconnect_timeout: Some(15),
                    max_connections: Some(25000),
                    threads: Some(available_parallelism().unwrap().get() as u64),
                    ssl: Some(false),
                    ssl_key: Some(String::from("")),
                    ssl_cert: Some(String::from("")),
                    tls_connection_rate: Some(256)
                }
            ),
            udp_server: vec!(
                UdpTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:6969"),
                    threads: Some(available_parallelism().unwrap().get() as u64),
                }
            ),
            api_server: vec!(
                ApiTrackersConfig {
                    enabled: true,
                    bind_address: String::from("0.0.0.0:8080"),
                    real_ip: Some(String::from("X-Real-IP")),
                    keep_alive: Some(60),
                    request_timeout: Some(30),
                    disconnect_timeout: Some(30),
                    max_connections: Some(25000),
                    threads: Some(available_parallelism().unwrap().get() as u64),
                    ssl: Some(false),
                    ssl_key: Some(String::from("")),
                    ssl_cert: Some(String::from("")),
                    tls_connection_rate: Some(256)
                }
            )
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

    pub fn validate(config: Configuration) {
        // Check Map
        let check_map = vec![
            ("[DB: torrents]", config.database_structure.clone().unwrap().torrents.unwrap().database_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: infohash", config.database_structure.clone().unwrap().torrents.unwrap().column_infohash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: seeds", config.database_structure.clone().unwrap().torrents.unwrap().column_seeds, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: peers", config.database_structure.clone().unwrap().torrents.unwrap().column_peers, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: torrents] Column: completed", config.database_structure.clone().unwrap().torrents.unwrap().column_completed, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: whitelist]", config.database_structure.clone().unwrap().whitelist.unwrap().database_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: whitelist] Column: infohash", config.database_structure.clone().unwrap().whitelist.unwrap().column_infohash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: blacklist]", config.database_structure.clone().unwrap().blacklist.unwrap().database_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: blacklist] Column: infohash", config.database_structure.clone().unwrap().blacklist.unwrap().column_infohash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: keys]", config.database_structure.clone().unwrap().keys.unwrap().database_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: keys] Column: hash", config.database_structure.clone().unwrap().keys.unwrap().column_hash, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: keys] Column: timeout", config.database_structure.clone().unwrap().keys.unwrap().column_timeout, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users]", config.database_structure.clone().unwrap().users.unwrap().database_name, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: id", config.database_structure.clone().unwrap().users.unwrap().column_id, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: uuid", config.database_structure.clone().unwrap().users.unwrap().column_uuid, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: key", config.database_structure.clone().unwrap().users.unwrap().column_key, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: uploaded", config.database_structure.clone().unwrap().users.unwrap().column_uploaded, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: downloaded", config.database_structure.clone().unwrap().users.unwrap().column_downloaded, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: completed", config.database_structure.clone().unwrap().users.unwrap().column_completed, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: active", config.database_structure.clone().unwrap().users.unwrap().column_active, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
            ("[DB: users] Column: updated", config.database_structure.clone().unwrap().users.unwrap().column_updated, r"^[a-z_][a-z0-9_]{0,30}$".to_string()),
        ];

        // Validation
        for (name, value, regex) in check_map {
            Self::validate_value(name, value, regex);
        }
    }

    pub fn validate_value(name: &str, value: String, regex: String)
    {
        let regex_check = Regex::new(regex.as_str()).unwrap();
        if !regex_check.is_match(value.as_str()){
            panic!("[VALIDATE CONFIG] Error checking {} [:] Name: \"{}\" [:] Regex: \"{}\"", name, value, regex_check);
        }
    }
}