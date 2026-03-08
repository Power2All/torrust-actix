mod common;

use std::sync::Arc;
use torrust_actix::ssl::enums::certificate_error::CertificateError;
use torrust_actix::ssl::enums::server_identifier::ServerIdentifier;
use torrust_actix::ssl::ssl::create_certificate_store;
use torrust_actix::ssl::structs::certificate_paths::CertificatePaths;
use torrust_actix::ssl::structs::certificate_store::CertificateStore;

#[tokio::test]
async fn test_server_identifier_http_tracker() {
    let id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    assert_eq!(id.bind_address(), "0.0.0.0:443");
    assert_eq!(id.server_type(), "http");
    assert_eq!(format!("{}", id), "HttpTracker(0.0.0.0:443)");
}

#[tokio::test]
async fn test_server_identifier_api_server() {
    let id = ServerIdentifier::ApiServer("0.0.0.0:8443".to_string());
    assert_eq!(id.bind_address(), "0.0.0.0:8443");
    assert_eq!(id.server_type(), "api");
    assert_eq!(format!("{}", id), "ApiServer(0.0.0.0:8443)");
}

#[tokio::test]
async fn test_server_identifier_websocket_master() {
    let id = ServerIdentifier::WebSocketMaster("0.0.0.0:9443".to_string());
    assert_eq!(id.bind_address(), "0.0.0.0:9443");
    assert_eq!(id.server_type(), "websocket");
    assert_eq!(format!("{}", id), "WebSocketMaster(0.0.0.0:9443)");
}

#[tokio::test]
async fn test_certificate_store_new() {
    let store = CertificateStore::new();
    assert!(store.all_servers().is_empty());
    assert!(store.get_all_certificates().is_empty());
}

#[tokio::test]
async fn test_certificate_store_default() {
    let store = CertificateStore::default();
    assert!(store.all_servers().is_empty());
}

#[tokio::test]
async fn test_certificate_store_thread_safety() {
    use std::thread;

    let store = Arc::new(CertificateStore::new());
    let mut handles = vec![];
    for i in 0..10 {
        let store_clone: Arc<CertificateStore> = Arc::clone(&store);
        let handle = thread::spawn(move || {
            let server_id = ServerIdentifier::HttpTracker(format!("0.0.0.0:{}", 443 + i));
            let _ = store_clone.get_certificate(&server_id);
            let _ = store_clone.get_paths(&server_id);
            let _ = store_clone.all_servers();
            let _ = store_clone.get_all_certificates();
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

#[tokio::test]
async fn test_certificate_error_display() {
    let err = CertificateError::CertFileNotFound("/path/to/cert.pem".to_string());
    assert!(err.to_string().contains("Certificate file not found"));
    let err = CertificateError::KeyFileNotFound("/path/to/key.pem".to_string());
    assert!(err.to_string().contains("Key file not found"));
    let err = CertificateError::NoKeyFound;
    assert!(err.to_string().contains("No private key found"));
}

#[tokio::test]
async fn test_certificate_paths() {
    let paths = CertificatePaths {
        cert_path: "/path/to/cert.pem".to_string(),
        key_path: "/path/to/key.pem".to_string(),
    };
    assert_eq!(paths.cert_path, "/path/to/cert.pem");
    assert_eq!(paths.key_path, "/path/to/key.pem");
}

#[tokio::test]
async fn test_server_identifier_equality() {
    let id1 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id2 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id3 = ServerIdentifier::HttpTracker("0.0.0.0:8443".to_string());
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[tokio::test]
async fn test_server_identifier_hash() {
    use std::collections::HashSet;
    let id1 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id2 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id3 = ServerIdentifier::ApiServer("0.0.0.0:443".to_string());
    let mut set = HashSet::new();
    set.insert(id1.clone());
    set.insert(id2);
    set.insert(id3.clone());
    assert_eq!(set.len(), 2);
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

mod test_tracker_ssl_integration {
    use super::*;

    #[tokio::test]
    async fn test_tracker_certificate_store_initialized() {
        let tracker: common::TestTracker = common::create_test_tracker().await;
        let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
        assert!(tracker.certificate_store.get_certificate(&server_id).is_none());
        let result = tracker.certificate_store.reload_certificate(&server_id);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_certificate_store_helper() {
        let store = create_certificate_store();
        assert!(store.all_servers().is_empty());
    }
}