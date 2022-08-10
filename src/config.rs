use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};
use toml;
use crate::common::CustomError;
use crate::databases::DatabaseDrivers;

#[derive(Debug)]
pub enum ConfigurationError {
    IOError(std::io::Error),
    ParseError(toml::de::Error)
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
    pub bind_address: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    pub log_level: String,
    pub log_console_interval: Option<u64>,
    pub statistics_enabled: bool,

    pub db_driver: DatabaseDrivers,
    pub db_path: String,
    pub persistency: bool,
    pub persistency_interval: Option<u64>,

    pub api_key: String,

    pub whitelist: bool,
    pub whitelist_from_persistency: bool,
    pub blacklist: bool,

    pub interval: Option<u64>,
    pub interval_minimum: Option<u64>,
    pub interval_cleanup: Option<u64>,
    pub peer_timeout: Option<u64>,
    pub peers_returned: Option<u64>,

    pub udp_server: Vec<UdpTrackersConfig>,
    pub http_server: Vec<HttpTrackersConfig>,
    pub api_server: Vec<ApiTrackersConfig>
}
impl Configuration {
    pub fn default() -> Configuration {
        let udp_server = vec!(
            UdpTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:6969")
            }
        );
        let http_server = vec!(
            HttpTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:6969"),
                ssl: false,
                ssl_key: String::from(""),
                ssl_cert: String::from("")
            }
        );
        let api_server = vec!(
            ApiTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:8080"),
                ssl: false,
                ssl_key: String::from(""),
                ssl_cert: String::from("")
            }
        );
        Configuration {
            log_level: String::from("info"),
            log_console_interval: Some(60),
            statistics_enabled: true,

            db_driver: DatabaseDrivers::SQLite3,
            db_path: String::from("sqlite://:memory:"),
            persistency: false,
            persistency_interval: Some(60),

            api_key: String::from("MyAccessToken"),

            whitelist: false,
            whitelist_from_persistency: false,
            blacklist: false,

            interval: Some(1800),
            interval_minimum: Some(1800),
            interval_cleanup: Some(900),
            peer_timeout: Some(2700),
            peers_returned: Some(200),

            udp_server,
            http_server,
            api_server
        }
    }

    pub fn load(data: &[u8]) -> Result<Configuration, toml::de::Error> {
        toml::from_slice(data)
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

    pub fn load_from_file() -> Result<Configuration, CustomError> {
        let mut config = Configuration::default();
        match Configuration::load_file("config.toml") {
            Ok(c) => { config = c; }
            Err(_) => {
                eprintln!("No config file found.");
                eprintln!("Creating config file..");

                let config_toml = toml::to_string(&config).unwrap();
                let save_file = Configuration::save_file("config.toml", config_toml);
                return match save_file {
                    Ok(_) => {
                        eprintln!("Please edit the config.TOML in the root folder, exitting now...");
                        Err(CustomError::new("create config.toml file"))
                    }
                    Err(e) => {
                        eprintln!("config.toml file could not be created, check permissions...");
                        eprintln!("{}", e);
                        Err(CustomError::new("could not create config.toml file"))
                    }
                }
            }
        };
        Ok(config)
    }
}