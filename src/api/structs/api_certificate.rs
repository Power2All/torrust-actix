use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Deserialize)]
pub struct CertificateReloadRequest {
    pub server_type: Option<String>,
    pub bind_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CertificateStatusItem {
    pub server_type: String,
    pub bind_address: String,
    pub cert_path: String,
    pub key_path: String,
    pub loaded_at: String,
}

#[derive(Debug, Serialize)]
pub struct CertificateReloadResult {
    pub server_type: String,
    pub bind_address: String,
    pub loaded_at: String,
}

#[derive(Debug, Serialize)]
pub struct CertificateReloadError {
    pub server_type: String,
    pub bind_address: String,
    pub error: String,
}