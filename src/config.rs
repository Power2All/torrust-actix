use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use toml;

use crate::common::CustomError;
use crate::databases::DatabaseDrivers;

#[derive(Debug)]
pub enum ConfigurationError {
    IOError(std::io::Error),
    ParseError(toml::de::Error),
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigurationError::IOError(e) => e.fmt(f),
            ConfigurationError::ParseError(e) => e.fmt(f)
        }
    }
}

impl std::error::Error for ConfigurationError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UdpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfig {
    pub db_torrents: String,
    pub table_torrents_info_hash: String,
    pub table_torrents_completed: String,
    pub db_whitelist: String,
    pub table_whitelist_info_hash: String,
    pub db_blacklist: String,
    pub table_blacklist_info_hash: String,
    pub db_keys: String,
    pub table_keys_hash: String,
    pub table_keys_timeout: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    pub log_level: String,
    pub log_console_interval: Option<u64>,
    pub statistics_enabled: bool,
    pub global_check_interval: Option<u64>,

    pub db_driver: DatabaseDrivers,
    pub db_path: String,
    pub persistence: bool,
    pub persistence_interval: Option<u64>,

    pub api_key: String,

    pub whitelist: bool,
    pub blacklist: bool,
    pub keys: bool,
    pub keys_cleanup_interval: Option<u64>,
    pub users: bool,

    pub maintenance_mode_enabled: bool,
    pub interval: Option<u64>,
    pub interval_minimum: Option<u64>,
    pub peer_timeout: Option<u64>,
    pub peers_returned: Option<u64>,

    pub interval_cleanup: Option<u64>,
    pub cleanup_chunks: Option<u64>,

    pub udp_server: Vec<UdpTrackersConfig>,
    pub http_server: Vec<HttpTrackersConfig>,
    pub api_server: Vec<ApiTrackersConfig>,

    pub db_structure: DatabaseStructureConfig,
}

impl Configuration {
    pub fn init() -> Configuration {
        let udp_server = vec!(
            UdpTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:6969"),
            }
        );
        let http_server = vec!(
            HttpTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:6969"),
                ssl: false,
                ssl_key: String::from(""),
                ssl_cert: String::from(""),
            }
        );
        let api_server = vec!(
            ApiTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:8080"),
                ssl: false,
                ssl_key: String::from(""),
                ssl_cert: String::from(""),
            }
        );
        Configuration {
            log_level: String::from("info"),
            log_console_interval: Some(60),
            statistics_enabled: true,
            global_check_interval: Some(10),

            db_driver: DatabaseDrivers::sqlite3,
            db_path: String::from("sqlite://:memory:"),
            persistence: false,
            persistence_interval: Some(60),

            api_key: String::from("MyAccessToken"),

            whitelist: false,
            blacklist: false,
            keys: false,
            keys_cleanup_interval: Some(60),
            users: false,

            maintenance_mode_enabled: false,
            interval: Some(1800),
            interval_minimum: Some(1800),
            peer_timeout: Some(2700),
            peers_returned: Some(200),

            interval_cleanup: Some(900),
            cleanup_chunks: Some(100000),

            udp_server,
            http_server,
            api_server,

            db_structure: DatabaseStructureConfig {
                db_torrents: String::from("torrents"),
                table_torrents_info_hash: String::from("info_hash"),
                table_torrents_completed: String::from("completed"),
                db_whitelist: String::from("whitelist"),
                table_whitelist_info_hash: String::from("info_hash"),
                db_blacklist: String::from("blacklist"),
                table_blacklist_info_hash: String::from("info_hash"),
                db_keys: String::from("keys"),
                table_keys_hash: String::from("hash"),
                table_keys_timeout: String::from("timeout"),
            },
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
                match file.write(data.as_ref()) {
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
            Err(_) => {
                eprintln!("No config file found or corrupt.");

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
        Ok(config)
    }
}