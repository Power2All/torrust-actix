use serde::{Deserialize, Serialize};
use crate::api::enums::cluster_mode::ClusterMode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackerConfig {
    pub api_key: String,
    pub whitelist_enabled: bool,
    pub blacklist_enabled: bool,
    pub keys_enabled: bool,
    pub keys_cleanup_interval: u64,
    pub users_enabled: bool,
    pub request_interval: u64,
    pub request_interval_minimum: u64,
    pub peers_timeout: u64,
    pub peers_cleanup_interval: u64,
    pub peers_cleanup_threads: u64,
    pub total_downloads: u64,
    pub swagger: bool,
    pub prometheus_id: String,
    pub cluster: ClusterMode
}