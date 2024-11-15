use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackerConfig {
    pub api_key: Option<String>,
    pub whitelist_enabled: Option<bool>,
    pub blacklist_enabled: Option<bool>,
    pub keys_enabled: Option<bool>,
    pub keys_cleanup_interval: Option<u64>,
    pub users_enabled: Option<bool>,
    pub request_interval: Option<u64>,
    pub request_interval_minimum: Option<u64>,
    pub peers_timeout: Option<u64>,
    pub peers_cleanup_interval: Option<u64>,
    pub total_downloads: u64,
    pub swagger: Option<bool>,
    pub sentry: Option<bool>,
    pub sentry_url: Option<String>
}