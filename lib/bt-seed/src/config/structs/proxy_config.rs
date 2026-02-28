use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProxyConfig {
    pub proxy_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}