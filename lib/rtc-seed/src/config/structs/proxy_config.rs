use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProxyConfig {
    pub proxy_type: String, // "http"|"http_auth"|"socks4"|"socks5"|"socks5_auth"
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}