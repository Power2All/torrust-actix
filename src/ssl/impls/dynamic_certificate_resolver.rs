use crate::ssl::enums::certificate_error::CertificateError;
use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::certificate_bundle::CertificateBundle;
use crate::ssl::structs::certificate_store::CertificateStore;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use rustls::server::ResolvesServerCert;
use std::sync::Arc;

impl std::fmt::Debug for DynamicCertificateResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicCertificateResolver")
            .field("server_id", &self.server_id)
            .field("has_cached_key", &self.cached_key.read().is_some())
            .finish()
    }
}

impl DynamicCertificateResolver {
    /// Creates a rustls certificate resolver that serves `server_id`'s certificate from the store.
    ///
    /// # Errors
    ///
    /// Returns a [`CertificateError`] when no certificate is loaded for the server, or when
    /// the bundle cannot be converted into a usable rustls signing key (`CertifiedKeyError`).
    pub fn new(
        store: Arc<CertificateStore>,
        server_id: ServerIdentifier,
    ) -> Result<Self, CertificateError> {
        let resolver = Self {
            store,
            server_id,
            cached_key: parking_lot::RwLock::new(None),
        };
        resolver.refresh_cache()?;
        Ok(resolver)
    }

    /// Returns the server identity this resolver serves certificates for.
    pub fn server_id(&self) -> &ServerIdentifier {
        &self.server_id
    }

    /// Refreshes the resolver's cached certified key from the store (after a hot reload).
    ///
    /// # Errors
    ///
    /// Returns a [`CertificateError`] when the store no longer holds a certificate, or when
    /// the bundle cannot be converted into a usable rustls signing key (`CertifiedKeyError`).
    pub fn refresh_cache(&self) -> Result<(), CertificateError> {
        let bundle = self
            .store
            .get_certificate(&self.server_id)
            .ok_or_else(|| CertificateError::ServerNotFound(self.server_id.to_string()))?;
        let certified_key = Self::bundle_to_certified_key(&bundle)?;
        *self.cached_key.write() = Some(Arc::new(certified_key));
        log::info!(
            "[CERTIFICATE] Refreshed certificate cache for {}",
            self.server_id
        );
        Ok(())
    }

    /// Returns `true` when the resolver currently holds a usable certificate.
    pub fn has_certificate(&self) -> bool {
        self.cached_key.read().is_some()
    }

    fn bundle_to_certified_key(bundle: &Arc<CertificateBundle>) -> Result<rustls::sign::CertifiedKey, CertificateError> {
        let signing_key = rustls::crypto::ring::sign::any_supported_type(&bundle.key)
            .map_err(|e| CertificateError::CertifiedKeyError(format!("{e}")))?;
        Ok(rustls::sign::CertifiedKey::new(bundle.certs.clone(), signing_key))
    }
}

impl ResolvesServerCert for DynamicCertificateResolver {
    fn resolve(&self, _client_hello: rustls::server::ClientHello<'_>) -> Option<Arc<rustls::sign::CertifiedKey>> {
        self.cached_key.read().clone()
    }
}