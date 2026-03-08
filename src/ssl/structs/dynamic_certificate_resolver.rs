use crate::ssl::enums::server_identifier::ServerIdentifier;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct DynamicCertificateResolver {
    pub(crate) store: Arc<crate::ssl::structs::certificate_store::CertificateStore>,
    pub(crate) server_id: ServerIdentifier,
    pub(crate) cached_key: RwLock<Option<Arc<rustls::sign::CertifiedKey>>>,
}