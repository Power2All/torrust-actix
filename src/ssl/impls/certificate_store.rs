use crate::common::structs::custom_error::CustomError;
use crate::security::security::validate_file_path;
use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::certificate_bundle::CertificateBundle;
use crate::ssl::structs::certificate_paths::CertificatePaths;
use crate::ssl::structs::certificate_store::CertificateStore;
use rustls::pki_types::pem::{
    Error as PemError,
    PemObject
};
use rustls::pki_types::{
    CertificateDer,
    PrivateKeyDer
};
use std::fs::File;

impl std::fmt::Debug for CertificateStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bundles = self.bundles.read();
        f.debug_struct("CertificateStore")
            .field("certificates_count", &bundles.len())
            .field("servers", &bundles.keys().collect::<Vec<_>>())
            .finish()
    }
}


impl CertificateStore {
    /// Creates an empty certificate store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a certificate chain and private key from the given PEM files and registers them
    /// under `server_id`.
    ///
    /// # Errors
    ///
    /// Returns a [`CertificateError`](crate::ssl::enums::certificate_error::CertificateError) when the files cannot be read or parsed.
    pub fn load_certificate(
        &self,
        server_id: ServerIdentifier,
        cert_path: &str,
        key_path: &str,
    ) -> Result<(), crate::ssl::enums::certificate_error::CertificateError> {
        let bundle = Self::load_bundle_from_files(cert_path, key_path)?;
        self.paths.write().insert(
            server_id.clone(),
            CertificatePaths {
                cert_path: cert_path.to_string(),
                key_path: key_path.to_string(),
            },
        );
        self.bundles.write().insert(server_id, std::sync::Arc::new(bundle));
        Ok(())
    }

    /// Returns the currently loaded certificate bundle for a server, if any.
    pub fn get_certificate(
        &self,
        server_id: &ServerIdentifier,
    ) -> Option<std::sync::Arc<CertificateBundle>> {
        self.bundles.read().get(server_id).cloned()
    }

    /// Re-reads a server's certificate from its registered file paths (hot reload).
    ///
    /// # Errors
    ///
    /// Returns a [`CertificateError`](crate::ssl::enums::certificate_error::CertificateError) when the server is unknown or the files are invalid.
    pub fn reload_certificate(
        &self,
        server_id: &ServerIdentifier,
    ) -> Result<(), crate::ssl::enums::certificate_error::CertificateError> {
        let paths = self
            .paths
            .read()
            .get(server_id)
            .cloned()
            .ok_or_else(|| crate::ssl::enums::certificate_error::CertificateError::ServerNotFound(server_id.to_string()))?;
        let bundle = Self::load_bundle_from_files(&paths.cert_path, &paths.key_path)?;
        self.bundles
            .write()
            .insert(server_id.clone(), std::sync::Arc::new(bundle));
        Ok(())
    }

    /// Returns the loaded certificate bundles of all registered servers.
    pub fn get_all_certificates(
        &self,
    ) -> Vec<(ServerIdentifier, std::sync::Arc<CertificateBundle>)> {
        self.bundles
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), std::sync::Arc::clone(v)))
            .collect()
    }

    fn load_bundle_from_files(
        cert_path: &str,
        key_path: &str,
    ) -> Result<CertificateBundle, crate::ssl::enums::certificate_error::CertificateError> {
        validate_file_path(cert_path)
            .map_err(|e: CustomError| crate::ssl::enums::certificate_error::CertificateError::CertFileNotFound(e.to_string()))?;
        validate_file_path(key_path)
            .map_err(|e: CustomError| crate::ssl::enums::certificate_error::CertificateError::KeyFileNotFound(e.to_string()))?;
        let key_file = File::open(key_path)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyFileNotFound(format!("{key_path}: {e}")))?;
        let certs_file = File::open(cert_path)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::CertFileNotFound(format!("{cert_path}: {e}")))?;
        let tls_certs: Vec<CertificateDer<'static>> = CertificateDer::pem_reader_iter(certs_file)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::CertParseError(e.to_string()))?;
        if tls_certs.is_empty() {
            return Err(crate::ssl::enums::certificate_error::CertificateError::CertParseError(
                "No certificates found in file".to_string(),
            ));
        }
        // One pass over the file: the PEM section kind picks PKCS#8, PKCS#1 or SEC1.
        let tls_key = PrivateKeyDer::from_pem_reader(key_file).map_err(|e| match e {
            PemError::NoItemsFound => crate::ssl::enums::certificate_error::CertificateError::NoKeyFound,
            other => crate::ssl::enums::certificate_error::CertificateError::KeyParseError(other.to_string()),
        })?;
        let signing_key = rustls::crypto::ring::sign::any_supported_type(&tls_key)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::CertifiedKeyError(format!("{e}")))?;
        let certified_key = std::sync::Arc::new(rustls::sign::CertifiedKey::new(tls_certs.clone(), signing_key));
        Ok(CertificateBundle {
            certs: tls_certs,
            key: tls_key,
            certified_key,
            loaded_at: chrono::Utc::now(),
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
        })
    }
}