mod common;

use std::sync::Arc;
use torrust_actix::ssl::certificate_store::{
    CertificateError, CertificatePaths, CertificateStore, ServerIdentifier,
};

#[tokio::test]
async fn test_server_identifier_http_tracker() {
    let id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    assert_eq!(id.bind_address(), "0.0.0.0:443");
    assert_eq!(id.server_type(), "http");
    assert_eq!(format!("{}", id), "HttpTracker(0.0.0.0:443)");
}

#[tokio::test]
async fn test_server_identifier_api_server() {
    let id = ServerIdentifier::ApiServer("127.0.0.1:8443".to_string());
    assert_eq!(id.bind_address(), "127.0.0.1:8443");
    assert_eq!(id.server_type(), "api");
    assert_eq!(format!("{}", id), "ApiServer(127.0.0.1:8443)");
}

#[tokio::test]
async fn test_server_identifier_websocket_master() {
    let id = ServerIdentifier::WebSocketMaster("[::]:9443".to_string());
    assert_eq!(id.bind_address(), "[::]:9443");
    assert_eq!(id.server_type(), "websocket");
    assert_eq!(format!("{}", id), "WebSocketMaster([::]:9443)");
}

#[tokio::test]
async fn test_server_identifier_equality() {
    let id1 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id2 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id3 = ServerIdentifier::HttpTracker("0.0.0.0:8443".to_string());
    let id4 = ServerIdentifier::ApiServer("0.0.0.0:443".to_string());
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
    assert_ne!(id1, id4);
}

#[tokio::test]
async fn test_server_identifier_clone() {
    let id1 = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let id2 = id1.clone();
    assert_eq!(id1, id2);
}

#[tokio::test]
async fn test_server_identifier_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(ServerIdentifier::HttpTracker("0.0.0.0:443".to_string()));
    set.insert(ServerIdentifier::ApiServer("0.0.0.0:443".to_string()));
    set.insert(ServerIdentifier::HttpTracker("0.0.0.0:443".to_string()));
    assert_eq!(set.len(), 2);
}

#[tokio::test]
async fn test_certificate_store_new() {
    let store = CertificateStore::new();
    assert!(store.all_servers().is_empty());
    assert!(store.get_all_certificates().is_empty());
}

#[tokio::test]
async fn test_certificate_store_default() {
    let store: CertificateStore = Default::default();
    assert!(store.all_servers().is_empty());
}

#[tokio::test]
async fn test_certificate_store_get_nonexistent() {
    let store = CertificateStore::new();
    let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    assert!(store.get_certificate(&server_id).is_none());
    assert!(store.get_paths(&server_id).is_none());
}

#[tokio::test]
async fn test_certificate_store_reload_nonexistent() {
    let store = CertificateStore::new();
    let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let result = store.reload_certificate(&server_id);
    assert!(result.is_err());
    match result {
        Err(CertificateError::ServerNotFound(id)) => {
            assert_eq!(id, server_id);
        }
        _ => panic!("Expected ServerNotFound error"),
    }
}

#[tokio::test]
async fn test_certificate_store_load_invalid_cert_path() {
    let store = CertificateStore::new();
    let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let result = store.load_certificate(
        server_id,
        "/nonexistent/path/cert.pem",
        "/nonexistent/path/key.pem",
    );
    assert!(result.is_err());
    match result {
        Err(CertificateError::KeyFileNotFound(_)) => {}
        Err(e) => panic!("Expected KeyFileNotFound, got: {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[tokio::test]
async fn test_certificate_store_debug() {
    let store = CertificateStore::new();
    let debug_str = format!("{:?}", store);
    assert!(debug_str.contains("CertificateStore"));
    assert!(debug_str.contains("certificates_count"));
}

#[tokio::test]
async fn test_certificate_store_thread_safety() {
    use std::thread;

    let store = Arc::new(CertificateStore::new());
    let mut handles = vec![];
    for i in 0..10 {
        let store_clone = Arc::clone(&store);
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
async fn test_certificate_store_reload_all_empty() {
    let store = CertificateStore::new();
    let results = store.reload_all();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_certificate_paths_clone() {
    let paths = CertificatePaths {
        cert_path: "/path/to/cert.pem".to_string(),
        key_path: "/path/to/key.pem".to_string(),
    };
    let cloned = paths.clone();
    assert_eq!(paths.cert_path, cloned.cert_path);
    assert_eq!(paths.key_path, cloned.key_path);
}

#[tokio::test]
async fn test_certificate_paths_debug() {
    let paths = CertificatePaths {
        cert_path: "/path/to/cert.pem".to_string(),
        key_path: "/path/to/key.pem".to_string(),
    };
    let debug_str = format!("{:?}", paths);
    assert!(debug_str.contains("cert_path"));
    assert!(debug_str.contains("key_path"));
}

#[tokio::test]
async fn test_certificate_error_cert_file_not_found() {
    let err = CertificateError::CertFileNotFound("/path/to/cert.pem".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Certificate file not found"));
    assert!(msg.contains("/path/to/cert.pem"));
}

#[tokio::test]
async fn test_certificate_error_key_file_not_found() {
    let err = CertificateError::KeyFileNotFound("/path/to/key.pem".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Key file not found"));
    assert!(msg.contains("/path/to/key.pem"));
}

#[tokio::test]
async fn test_certificate_error_cert_parse_error() {
    let err = CertificateError::CertParseError("invalid PEM format".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Failed to parse certificate"));
    assert!(msg.contains("invalid PEM format"));
}

#[tokio::test]
async fn test_certificate_error_key_parse_error() {
    let err = CertificateError::KeyParseError("invalid key format".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Failed to parse key"));
    assert!(msg.contains("invalid key format"));
}

#[tokio::test]
async fn test_certificate_error_no_key_found() {
    let err = CertificateError::NoKeyFound;
    let msg = err.to_string();
    assert!(msg.contains("No private key found"));
}

#[tokio::test]
async fn test_certificate_error_certified_key_error() {
    let err = CertificateError::CertifiedKeyError("signing error".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Failed to build certified key"));
    assert!(msg.contains("signing error"));
}

#[tokio::test]
async fn test_certificate_error_server_not_found() {
    let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    let err = CertificateError::ServerNotFound(server_id);
    let msg = err.to_string();
    assert!(msg.contains("Server not found"));
    assert!(msg.contains("HttpTracker"));
}

#[tokio::test]
async fn test_certificate_error_debug() {
    let err = CertificateError::NoKeyFound;
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("NoKeyFound"));
}

use actix_web::{test, web, App};
use torrust_actix::api::api_certificate::{
    api_service_certificate_reload, api_service_certificate_status,
};
use torrust_actix::api::structs::api_service_data::ApiServiceData;

#[actix_web::test]
async fn test_api_certificate_status_empty() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route(
                "/api/certificate/status",
                web::get().to(api_service_certificate_status),
            ),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/api/certificate/status?token=MyApiKey")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Certificate status endpoint should return 200"
    );
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["certificates"].is_array());
    assert_eq!(json["certificates"].as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn test_api_certificate_reload_empty() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route(
                "/api/certificate/reload",
                web::post().to(api_service_certificate_reload),
            ),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/certificate/reload?token=MyApiKey")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Certificate reload endpoint should return 200"
    );
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "no_certificates");
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("No SSL certificates"));
}

#[actix_web::test]
async fn test_api_certificate_status_requires_token() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route(
                "/api/certificate/status",
                web::get().to(api_service_certificate_status),
            ),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/api/certificate/status")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
        "Certificate status should require authentication"
    );
}

#[actix_web::test]
async fn test_api_certificate_reload_requires_token() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route(
                "/api/certificate/reload",
                web::post().to(api_service_certificate_reload),
            ),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/certificate/reload")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
        "Certificate reload should require authentication"
    );
}

#[actix_web::test]
async fn test_api_certificate_reload_with_filter() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route(
                "/api/certificate/reload",
                web::post().to(api_service_certificate_reload),
            ),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/certificate/reload?token=MyApiKey")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .set_json(serde_json::json!({
            "server_type": "http",
            "bind_address": "0.0.0.0:443"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Certificate reload with filter should return 200"
    );
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "no_certificates");
}

#[actix_web::test]
async fn test_tracker_has_certificate_store() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    assert!(tracker.certificate_store.all_servers().is_empty());
    assert!(tracker.certificate_store.get_all_certificates().is_empty());
}

#[actix_web::test]
async fn test_certificate_store_from_tracker() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let server_id = ServerIdentifier::HttpTracker("0.0.0.0:443".to_string());
    assert!(tracker.certificate_store.get_certificate(&server_id).is_none());
    let result = tracker.certificate_store.reload_certificate(&server_id);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_certificate_store_helper() {
    let store = torrust_actix::ssl::certificate_store::create_certificate_store();
    assert!(store.all_servers().is_empty());
}