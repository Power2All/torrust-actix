use crate::config::structs::proxy_config::ProxyConfig;
use serde::{
    Deserialize,
    Serialize
};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub web_port: Option<u16>,
    pub web_password: Option<String>,
    pub web_cert: Option<PathBuf>,
    pub web_key: Option<PathBuf>,
    pub proxy: Option<ProxyConfig>,
    /// Minimum log level: "error" | "warn" | "info" (default) | "debug" | "trace"
    pub log_level: Option<String>,
    /// Print periodic upload stats every 10 s (default: true)
    pub show_stats: Option<bool>,
}