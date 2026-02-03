use super::certificate_store::{CertificateBundle, CertificateError, CertificateStore, ServerIdentifier};
use parking_lot::RwLock;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::sync::Arc;

pub struct DynamicCertificateResolver {
    store: Arc<CertificateStore>,
    server_id: ServerIdentifier,
    cached_key: RwLock<Option<Arc<CertifiedKey>>>,
}

impl std::fmt::Debug for DynamicCertificateResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicCertificateResolver")
            .field("server_id", &self.server_id)
            .field("has_cached_key", &self.cached_key.read().is_some())
            .finish()
    }
}

impl DynamicCertificateResolver {
    pub fn new(
        store: Arc<CertificateStore>,
        server_id: ServerIdentifier,
    ) -> Result<Self, CertificateError> {
        let resolver = Self {
            store,
            server_id,
            cached_key: RwLock::new(None),
        };
        resolver.refresh_cache()?;
        Ok(resolver)
    }

    pub fn server_id(&self) -> &ServerIdentifier {
        &self.server_id
    }

    pub fn refresh_cache(&self) -> Result<(), CertificateError> {
        let bundle = self
            .store
            .get_certificate(&self.server_id)
            .ok_or_else(|| CertificateError::ServerNotFound(self.server_id.clone()))?;
        let certified_key = Self::bundle_to_certified_key(&bundle)?;
        *self.cached_key.write() = Some(Arc::new(certified_key));
        log::info!(
            "[CERTIFICATE] Refreshed certificate cache for {}",
            self.server_id
        );
        Ok(())
    }

    pub fn has_certificate(&self) -> bool {
        self.cached_key.read().is_some()
    }

    fn bundle_to_certified_key(bundle: &Arc<CertificateBundle>) -> Result<CertifiedKey, CertificateError> {
        let signing_key = rustls::crypto::ring::sign::any_supported_type(&bundle.key)
            .map_err(|e| CertificateError::CertifiedKeyError(format!("{}", e)))?;
        Ok(CertifiedKey::new(bundle.certs.clone(), signing_key))
    }
}

impl ResolvesServerCert for DynamicCertificateResolver {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        self.cached_key.read().clone()
    }
}

pub fn create_server_config_with_resolver(
    resolver: Arc<DynamicCertificateResolver>,
) -> rustls::ServerConfig {
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssl::certificate_store::create_certificate_store;

    #[test]
    fn test_resolver_creation_without_cert() {
        let store = create_certificate_store();
        let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
        let result = DynamicCertificateResolver::new(store, server_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_server_identifier_methods() {
        let server_id = ServerIdentifier::ApiServer("0.0.0.0:8443".to_string());
        assert_eq!(server_id.bind_address(), "0.0.0.0:8443");
        assert_eq!(server_id.server_type(), "api");
    }
}