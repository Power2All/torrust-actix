use serde::{Deserialize, Serialize};
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::database_structure_config::DatabaseStructureConfig;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;
use crate::database::enums::database_drivers::DatabaseDrivers;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    pub log_level: String,
    pub log_console_interval: Option<u64>,

    pub db_driver: DatabaseDrivers,
    pub db_path: String,
    pub persistence: bool,
    pub persistence_interval: Option<u64>,
    pub total_downloads: u64,

    pub api_key: String,

    pub whitelist: bool,
    pub blacklist: bool,
    pub keys: bool,
    pub keys_cleanup_interval: Option<u64>,
    pub users: bool,

    /* Peer Configuration */
    pub interval: Option<u64>,
    pub interval_minimum: Option<u64>,
    pub peer_timeout: Option<u64>,
    pub peers_returned: Option<u64>,

    pub http_keep_alive: u64,
    pub http_request_timeout: u64,
    pub http_disconnect_timeout: u64,
    pub api_keep_alive: u64,
    pub api_request_timeout: u64,
    pub api_disconnect_timeout: u64,

    pub interval_cleanup: Option<u64>,
    pub cleanup_chunks: Option<u64>,

    pub http_server: Vec<HttpTrackersConfig>,
    pub udp_server: Vec<UdpTrackersConfig>,
    pub web_support: bool,
    pub api_server: Vec<ApiTrackersConfig>,
    pub http_real_ip: String,

    pub db_structure: DatabaseStructureConfig,
}
