use crate::ssl::structs::certificate_store::CertificateStore;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use std::sync::Arc;

pub fn create_certificate_store() -> Arc<CertificateStore> {
    Arc::new(CertificateStore::new())
}

pub fn create_server_config_with_resolver(
    resolver: Arc<DynamicCertificateResolver>,
) -> rustls::ServerConfig {
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver)
}