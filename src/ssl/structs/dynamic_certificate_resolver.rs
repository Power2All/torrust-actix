use crate::ssl::enums::server_identifier::ServerIdentifier;
use std::sync::Arc;

pub struct DynamicCertificateResolver {
    pub(crate) store: Arc<crate::ssl::structs::certificate_store::CertificateStore>,
    pub(crate) server_id: ServerIdentifier,
}