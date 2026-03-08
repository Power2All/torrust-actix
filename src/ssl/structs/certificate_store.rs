use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::certificate_paths::CertificatePaths;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CertificateStore {
    pub(crate) bundles: RwLock<HashMap<ServerIdentifier, Arc<crate::ssl::structs::certificate_bundle::CertificateBundle>>>,
    pub(crate) paths: RwLock<HashMap<ServerIdentifier, CertificatePaths>>,
}