use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CertificateReloadRequest {
    pub server_type: Option<String>,
    pub bind_address: Option<String>,
}