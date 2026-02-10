use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::certificate_bundle::CertificateBundle;
use crate::ssl::structs::certificate_paths::CertificatePaths;
use crate::ssl::structs::certificate_store::CertificateStore;
use rustls::pki_types::{
    CertificateDer,
    PrivateKeyDer
};
use std::fs::File;
use std::io::BufReader;

impl std::fmt::Debug for CertificateStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bundles = self.bundles.read();
        f.debug_struct("CertificateStore")
            .field("certificates_count", &bundles.len())
            .field("servers", &bundles.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for CertificateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CertificateStore {
    pub fn new() -> Self {
        Self {
            bundles: parking_lot::RwLock::new(std::collections::HashMap::new()),
            paths: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

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

    pub fn get_certificate(
        &self,
        server_id: &ServerIdentifier,
    ) -> Option<std::sync::Arc<CertificateBundle>> {
        self.bundles.read().get(server_id).cloned()
    }

    pub fn get_paths(&self, server_id: &ServerIdentifier) -> Option<CertificatePaths> {
        self.paths.read().get(server_id).cloned()
    }

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

    pub fn reload_certificate_with_paths(
        &self,
        server_id: &ServerIdentifier,
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
        self.bundles
            .write()
            .insert(server_id.clone(), std::sync::Arc::new(bundle));
        Ok(())
    }

    pub fn all_servers(&self) -> Vec<(ServerIdentifier, CertificatePaths)> {
        self.paths
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn get_all_certificates(
        &self,
    ) -> Vec<(ServerIdentifier, std::sync::Arc<CertificateBundle>)> {
        self.bundles
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), std::sync::Arc::clone(v)))
            .collect()
    }

    pub fn reload_all(&self) -> Vec<(ServerIdentifier, Result<(), crate::ssl::enums::certificate_error::CertificateError>)> {
        let servers: Vec<_> = self.paths.read().keys().cloned().collect();
        servers
            .into_iter()
            .map(|server_id| {
                let result = self.reload_certificate(&server_id);
                (server_id, result)
            })
            .collect()
    }

    fn load_bundle_from_files(
        cert_path: &str,
        key_path: &str,
    ) -> Result<CertificateBundle, crate::ssl::enums::certificate_error::CertificateError> {
        let key_file = File::open(key_path)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyFileNotFound(format!("{}: {}", key_path, e)))?;
        let mut key_reader = BufReader::new(key_file);
        let certs_file = File::open(cert_path)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::CertFileNotFound(format!("{}: {}", cert_path, e)))?;
        let mut certs_reader = BufReader::new(certs_file);
        let tls_certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut certs_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::CertParseError(e.to_string()))?;
        if tls_certs.is_empty() {
            return Err(crate::ssl::enums::certificate_error::CertificateError::CertParseError(
                "No certificates found in file".to_string(),
            ));
        }
        let tls_key = Self::parse_private_key(&mut key_reader, key_path)?;
        Ok(CertificateBundle {
            certs: tls_certs,
            key: tls_key,
            loaded_at: chrono::Utc::now(),
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
        })
    }

    fn parse_private_key(
        reader: &mut BufReader<File>,
        key_path: &str,
    ) -> Result<PrivateKeyDer<'static>, crate::ssl::enums::certificate_error::CertificateError> {
        if let Some(key_result) = rustls_pemfile::pkcs8_private_keys(reader).next() {
            return key_result
                .map(PrivateKeyDer::Pkcs8)
                .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyParseError(e.to_string()));
        }
        let key_file = File::open(key_path)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyFileNotFound(format!("{}: {}", key_path, e)))?;
        let mut reader = BufReader::new(key_file);
        if let Some(key_result) = rustls_pemfile::rsa_private_keys(&mut reader).next() {
            return key_result
                .map(PrivateKeyDer::Pkcs1)
                .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyParseError(e.to_string()));
        }
        let key_file = File::open(key_path)
            .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyFileNotFound(format!("{}: {}", key_path, e)))?;
        let mut reader = BufReader::new(key_file);
        if let Some(key_result) = rustls_pemfile::ec_private_keys(&mut reader).next() {
            return key_result
                .map(PrivateKeyDer::Sec1)
                .map_err(|e| crate::ssl::enums::certificate_error::CertificateError::KeyParseError(e.to_string()));
        }
        Err(crate::ssl::enums::certificate_error::CertificateError::NoKeyFound)
    }
}