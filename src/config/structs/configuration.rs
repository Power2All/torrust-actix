use serde::{Deserialize, Serialize};
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::cache_config::CacheConfig;
use crate::config::structs::database_config::DatabaseConfig;
use crate::config::structs::database_structure_config::DatabaseStructureConfig;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::config::structs::sentry_config::SentryConfig;
use crate::config::structs::tracker_config::TrackerConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    pub log_level: String,
    pub log_console_interval: u64,
    pub tracker_config: TrackerConfig,
    pub sentry_config: SentryConfig,
    pub database: DatabaseConfig,
    pub database_structure: DatabaseStructureConfig,
    #[serde(default)]
    pub cache: Option<CacheConfig>,
    pub http_server: Vec<HttpTrackersConfig>,
    pub udp_server: Vec<UdpTrackersConfig>,
    pub api_server: Vec<ApiTrackersConfig>
}