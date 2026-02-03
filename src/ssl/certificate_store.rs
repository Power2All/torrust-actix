//! Certificate storage with hot-reload support.
//!
//! This module provides thread-safe certificate storage and management,
//! allowing certificates to be loaded, retrieved, and reloaded at runtime.

use parking_lot::RwLock;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use thiserror::Error;

/// A loaded certificate bundle containing the certificate chain and private key.
///
/// This struct holds the parsed certificate data ready for use with rustls,
/// along with metadata about when and from where it was loaded.
pub struct CertificateBundle {
    pub certs: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub cert_path: String,
    pub key_path: String,
}

impl std::fmt::Debug for CertificateBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertificateBundle")
            .field("certs_count", &self.certs.len())
            .field("cert_path", &self.cert_path)
            .field("key_path", &self.key_path)
            .field("loaded_at", &self.loaded_at)
            .finish()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ServerIdentifier {
    HttpTracker(String),
    ApiServer(String),
    WebSocketMaster(String),
}

impl ServerIdentifier {
    pub fn bind_address(&self) -> &str {
        match self {
            ServerIdentifier::HttpTracker(addr) => addr,
            ServerIdentifier::ApiServer(addr) => addr,
            ServerIdentifier::WebSocketMaster(addr) => addr,
        }
    }

    pub fn server_type(&self) -> &'static str {
        match self {
            ServerIdentifier::HttpTracker(_) => "http",
            ServerIdentifier::ApiServer(_) => "api",
            ServerIdentifier::WebSocketMaster(_) => "websocket",
        }
    }
}

impl std::fmt::Display for ServerIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerIdentifier::HttpTracker(addr) => {
                write!(f, "HttpTracker({})", addr)
            }
            ServerIdentifier::ApiServer(addr) => {
                write!(f, "ApiServer({})", addr)
            }
            ServerIdentifier::WebSocketMaster(addr) => {
                write!(f, "WebSocketMaster({})", addr)
            }
        }
    }
}

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
    ServerNotFound(ServerIdentifier),
}

#[derive(Debug, Clone)]
pub struct CertificatePaths {
    pub cert_path: String,
    pub key_path: String,
}

pub struct CertificateStore {
    bundles: RwLock<HashMap<ServerIdentifier, Arc<CertificateBundle>>>,
    paths: RwLock<HashMap<ServerIdentifier, CertificatePaths>>,
}

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
            bundles: RwLock::new(HashMap::new()),
            paths: RwLock::new(HashMap::new()),
        }
    }

    pub fn load_certificate(
        &self,
        server_id: ServerIdentifier,
        cert_path: &str,
        key_path: &str,
    ) -> Result<(), CertificateError> {
        let bundle = Self::load_bundle_from_files(cert_path, key_path)?;
        self.paths.write().insert(
            server_id.clone(),
            CertificatePaths {
                cert_path: cert_path.to_string(),
                key_path: key_path.to_string(),
            },
        );
        self.bundles.write().insert(server_id, Arc::new(bundle));
        Ok(())
    }

    pub fn get_certificate(&self, server_id: &ServerIdentifier) -> Option<Arc<CertificateBundle>> {
        self.bundles.read().get(server_id).cloned()
    }

    pub fn get_paths(&self, server_id: &ServerIdentifier) -> Option<CertificatePaths> {
        self.paths.read().get(server_id).cloned()
    }

    pub fn reload_certificate(
        &self,
        server_id: &ServerIdentifier,
    ) -> Result<(), CertificateError> {
        let paths = self
            .paths
            .read()
            .get(server_id)
            .cloned()
            .ok_or_else(|| CertificateError::ServerNotFound(server_id.clone()))?;
        let bundle = Self::load_bundle_from_files(&paths.cert_path, &paths.key_path)?;
        self.bundles
            .write()
            .insert(server_id.clone(), Arc::new(bundle));
        Ok(())
    }

    pub fn reload_certificate_with_paths(
        &self,
        server_id: &ServerIdentifier,
        cert_path: &str,
        key_path: &str,
    ) -> Result<(), CertificateError> {
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
            .insert(server_id.clone(), Arc::new(bundle));
        Ok(())
    }

    pub fn all_servers(&self) -> Vec<(ServerIdentifier, CertificatePaths)> {
        self.paths
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn get_all_certificates(&self) -> Vec<(ServerIdentifier, Arc<CertificateBundle>)> {
        self.bundles
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }

    pub fn reload_all(&self) -> Vec<(ServerIdentifier, Result<(), CertificateError>)> {
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
    ) -> Result<CertificateBundle, CertificateError> {
        let key_file = File::open(key_path)
            .map_err(|e| CertificateError::KeyFileNotFound(format!("{}: {}", key_path, e)))?;
        let mut key_reader = BufReader::new(key_file);
        let certs_file = File::open(cert_path)
            .map_err(|e| CertificateError::CertFileNotFound(format!("{}: {}", cert_path, e)))?;
        let mut certs_reader = BufReader::new(certs_file);
        let tls_certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut certs_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CertificateError::CertParseError(e.to_string()))?;
        if tls_certs.is_empty() {
            return Err(CertificateError::CertParseError(
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
    ) -> Result<PrivateKeyDer<'static>, CertificateError> {
        if let Some(key_result) = rustls_pemfile::pkcs8_private_keys(reader).next() {
            return key_result
                .map(PrivateKeyDer::Pkcs8)
                .map_err(|e| CertificateError::KeyParseError(e.to_string()));
        }
        let key_file = File::open(key_path)
            .map_err(|e| CertificateError::KeyFileNotFound(format!("{}: {}", key_path, e)))?;
        let mut reader = BufReader::new(key_file);
        if let Some(key_result) = rustls_pemfile::rsa_private_keys(&mut reader).next() {
            return key_result
                .map(PrivateKeyDer::Pkcs1)
                .map_err(|e| CertificateError::KeyParseError(e.to_string()));
        }
        let key_file = File::open(key_path)
            .map_err(|e| CertificateError::KeyFileNotFound(format!("{}: {}", key_path, e)))?;
        let mut reader = BufReader::new(key_file);
        if let Some(key_result) = rustls_pemfile::ec_private_keys(&mut reader).next() {
            return key_result
                .map(PrivateKeyDer::Sec1)
                .map_err(|e| CertificateError::KeyParseError(e.to_string()));
        }
        Err(CertificateError::NoKeyFound)
    }
}

pub fn create_certificate_store() -> Arc<CertificateStore> {
    Arc::new(CertificateStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_identifier_display() {
        let http = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
        assert_eq!(format!("{}", http), "HttpTracker(0.0.0.0:443)");
        let api = ServerIdentifier::ApiServer("0.0.0.0:8443".to_string());
        assert_eq!(format!("{}", api), "ApiServer(0.0.0.0:8443)");
        let ws = ServerIdentifier::WebSocketMaster("0.0.0.0:9443".to_string());
        assert_eq!(format!("{}", ws), "WebSocketMaster(0.0.0.0:9443)");
    }

    #[test]
    fn test_certificate_store_new() {
        let store = CertificateStore::new();
        assert!(store.all_servers().is_empty());
    }

    #[test]
    fn test_server_identifier_equality() {
        let id1 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
        let id2 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
        let id3 = ServerIdentifier::HttpTracker("0.0.0.0:8443".to_string());
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_certificate_error_display() {
        let err = CertificateError::CertFileNotFound("/path/to/cert.pem".to_string());
        assert!(err.to_string().contains("Certificate file not found"));
        let err = CertificateError::NoKeyFound;
        assert!(err.to_string().contains("No private key found"));
    }

    #[test]
    fn test_server_identifier_methods() {
        let http = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
        assert_eq!(http.bind_address(), "0.0.0.0:443");
        assert_eq!(http.server_type(), "http");
        let api = ServerIdentifier::ApiServer("0.0.0.0:8443".to_string());
        assert_eq!(api.bind_address(), "0.0.0.0:8443");
        assert_eq!(api.server_type(), "api");
    }
}