use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub real_ip: Option<String>,
    pub keep_alive: Option<u64>,
    pub request_timeout: Option<u64>,
    pub disconnect_timeout: Option<u64>,
    pub max_connections: Option<u64>,
    pub threads: Option<u64>,
    pub ssl: Option<bool>,
    pub ssl_key: Option<String>,
    pub ssl_cert: Option<String>,
    pub tls_connection_rate: Option<u64>
}