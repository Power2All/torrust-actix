use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WebConfig {
    pub port: u16,
    pub password: Option<String>,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}