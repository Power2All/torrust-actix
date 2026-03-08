use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CertificateReloadResult {
    pub server_type: String,
    pub bind_address: String,
    pub loaded_at: String,
}