use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub threads: Option<u64>,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String,
}
