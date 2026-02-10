use rustls::pki_types::{
    CertificateDer,
    PrivateKeyDer
};

pub struct CertificateBundle {
    pub certs: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub cert_path: String,
    pub key_path: String,
}