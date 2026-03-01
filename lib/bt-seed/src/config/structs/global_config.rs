use crate::config::structs::proxy_config::ProxyConfig;
use serde::{
    Deserialize,
    Serialize
};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub listen_port: Option<u16>,
    pub web_port: Option<u16>,
    pub web_password: Option<String>,
    pub web_cert: Option<PathBuf>,
    pub web_key: Option<PathBuf>,
    pub proxy: Option<ProxyConfig>,
    pub upnp: Option<bool>,
    pub log_level: Option<String>,
    pub show_stats: Option<bool>,
}