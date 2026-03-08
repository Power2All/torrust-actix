use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CertificateReloadError {
    pub server_type: String,
    pub bind_address: String,
    pub error: String,
}