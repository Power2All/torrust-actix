use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
use serde::{
    Deserialize,
    Serialize
};

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
    pub cluster: ClusterMode,
    pub cluster_encoding: ClusterEncoding,
    pub cluster_token: String,
    pub cluster_bind_address: String,
    pub cluster_master_address: String,
    pub cluster_keep_alive: u64,
    pub cluster_request_timeout: u64,
    pub cluster_disconnect_timeout: u64,
    pub cluster_reconnect_interval: u64,
    pub cluster_max_connections: u64,
    pub cluster_threads: u64,
    pub cluster_ssl: bool,
    pub cluster_ssl_key: String,
    pub cluster_ssl_cert: String,
    pub cluster_tls_connection_rate: u64,
}