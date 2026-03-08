use crate::ssl::structs::certificate_bundle::CertificateBundle;

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