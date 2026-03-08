use thiserror::Error;

#[derive(Debug, Error)]
pub enum CertificateError {
    #[error("Certificate file not found: {0}")]
    CertFileNotFound(String),
    #[error("Key file not found: {0}")]
    KeyFileNotFound(String),
    #[error("Failed to parse certificate: {0}")]
    CertParseError(String),
    #[error("Failed to parse key: {0}")]
    KeyParseError(String),
    #[error("No private key found in file")]
    NoKeyFound,
    #[error("Failed to build certified key: {0}")]
    CertifiedKeyError(String),
    #[error("Server not found: {0}")]
    ServerNotFound(String),
}