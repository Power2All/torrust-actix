use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub real_ip: String,
    pub keep_alive: u64,
    pub request_timeout: u64,
    pub disconnect_timeout: u64,
    pub max_connections: u64,
    pub threads: u64,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String,
    pub tls_connection_rate: u64
}