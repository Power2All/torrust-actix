use crate::ssl::structs::certificate_store::CertificateStore;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use std::sync::Arc;

/// Creates the shared hot-reloadable certificate store used by all TLS listeners.
pub fn create_certificate_store() -> Arc<CertificateStore> {
    Arc::new(CertificateStore::new())
}

/// Builds a rustls server configuration that resolves certificates dynamically from the
/// store, so listeners pick up reloaded certificates without restarting.
///
/// # Errors
///
/// Returns a [`CertificateError`](crate::ssl::enums::certificate_error::CertificateError) when no certificate is available for the server.
pub fn create_server_config_with_resolver(
    resolver: Arc<DynamicCertificateResolver>,
) -> rustls::ServerConfig {
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver)
}