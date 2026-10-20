use rustls::pki_types::{
    CertificateDer,
    PrivateKeyDer
};
use rustls::sign::CertifiedKey;
use std::sync::Arc;

pub struct CertificateBundle {
    pub certs: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
    /// What TLS handshakes are served from; built when the files load so a bad key fails the load
    /// (or the reload) instead of the handshake.
    pub certified_key: Arc<CertifiedKey>,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub cert_path: String,
    pub key_path: String,
}