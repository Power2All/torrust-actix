use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTrackersConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl: bool,
    pub ssl_key: String,
    pub ssl_cert: String,
}
