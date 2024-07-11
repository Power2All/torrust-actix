use std::fs::File;
use std::io::Write;
use crate::common::structs::custom_error::CustomError;
use crate::config::enums::configuration_error::ConfigurationError;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::configuration::Configuration;
use crate::config::structs::database_structure_config::DatabaseStructureConfig;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;
use crate::database::enums::database_drivers::DatabaseDrivers;

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
                threads: Some(0),
                ssl: false,
                ssl_key: String::from(""),
                ssl_cert: String::from(""),
            }
        );
        let api_server = vec!(
            ApiTrackersConfig {
                enabled: false,
                bind_address: String::from("0.0.0.0:8080"),
                threads: Some(0),
                ssl: false,
                ssl_key: String::from(""),
                ssl_cert: String::from(""),
            }
        );
        Configuration {
            log_level: String::from("info"),
            log_console_interval: Some(60),
            log_perf_count: Some(10000),

            db_driver: DatabaseDrivers::sqlite3,
            db_path: String::from("sqlite://:memory:"),
            persistence: false,
            persistence_interval: Some(60),
            total_downloads: 0,

            api_key: String::from("MyAccessToken"),

            whitelist: false,
            blacklist: false,
            keys: false,
            keys_cleanup_interval: Some(60),
            users: false,

            interval: Some(1800),
            interval_minimum: Some(1800),
            peer_timeout: Some(2700),
            peers_returned: Some(200),

            http_keep_alive: 60,
            http_request_timeout: 10,
            http_disconnect_timeout: 10,
            api_keep_alive: 60,
            api_request_timeout: 60,
            api_disconnect_timeout: 60,

            interval_cleanup: Some(900),
            cleanup_chunks: Some(100000),

            http_server,
            udp_server,
            web_support: false,
            api_server,
            http_real_ip: String::from("X-Real-IP"),

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
                db_users: String::from("users"),
                table_users_uuid: String::from("uuid"),
                table_users_key: String::from("key"),
                table_users_uploaded: String::from("uploaded"),
                table_users_downloaded: String::from("downloaded"),
                table_users_completed: String::from("completed"),
                table_users_updated: String::from("updated"),
                table_users_active: String::from("active"),
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
