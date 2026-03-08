#[cfg(test)]
mod ssl_tests {
    use crate::ssl::structs::certificate_store::CertificateStore;

    #[test]
    fn test_certificate_store_new() {
        let store = CertificateStore::new();
        assert!(store.all_servers().is_empty());
    }
}