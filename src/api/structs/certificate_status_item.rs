use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CertificateStatusItem {
    pub server_type: String,
    pub bind_address: String,
    pub cert_path: String,
    pub key_path: String,
    pub loaded_at: String,
}