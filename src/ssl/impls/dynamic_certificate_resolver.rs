use crate::ssl::enums::certificate_error::CertificateError;
use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::certificate_store::CertificateStore;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use rustls::server::ResolvesServerCert;
use std::sync::Arc;

impl std::fmt::Debug for DynamicCertificateResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicCertificateResolver")
            .field("server_id", &self.server_id)
            .finish()
    }
}

impl DynamicCertificateResolver {
    /// Creates a rustls certificate resolver that serves `server_id`'s certificate from the store.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::ServerNotFound`] when no certificate is loaded for the server.
    pub fn new(
        store: Arc<CertificateStore>,
        server_id: ServerIdentifier,
    ) -> Result<Self, CertificateError> {
        if store.get_certificate(&server_id).is_none() {
            return Err(CertificateError::ServerNotFound(server_id.to_string()));
        }
        Ok(Self { store, server_id })
    }

    /// Loads `cert_path`/`key_path` into `store` for `server_id` and returns a rustls config that
    /// serves them through a resolver, so `POST /api/certificate/reload` reaches new handshakes.
    ///
    /// # Errors
    ///
    /// Returns the [`CertificateError`] from loading the certificate.
    pub fn server_config(
        store: &Arc<CertificateStore>,
        server_id: ServerIdentifier,
        cert_path: &str,
        key_path: &str,
    ) -> Result<rustls::ServerConfig, CertificateError> {
        store.load_certificate(server_id.clone(), cert_path, key_path)?;
        let resolver = Self::new(Arc::clone(store), server_id)?;
        Ok(rustls::ServerConfig::builder().with_no_client_auth().with_cert_resolver(Arc::new(resolver)))
    }

    /// Returns the key the next TLS handshake will be served. It is read from the store every
    /// time, so a hot reload takes effect on the next connection; a copy cached at startup kept
    /// serving the old certificate until a restart.
    pub fn current_key(&self) -> Option<Arc<rustls::sign::CertifiedKey>> {
        self.store
            .get_certificate(&self.server_id)
            .map(|bundle| Arc::clone(&bundle.certified_key))
    }
}

impl ResolvesServerCert for DynamicCertificateResolver {
    fn resolve(&self, _client_hello: rustls::server::ClientHello<'_>) -> Option<Arc<rustls::sign::CertifiedKey>> {
        self.current_key()
    }
}