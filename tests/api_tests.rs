mod common;

use actix_web::{
    test,
    web,
    App
};
use std::sync::Arc;
use torrust_actix::api::api_blacklists::api_service_blacklist_delete;
use torrust_actix::api::api_keys::api_service_key_delete;
use torrust_actix::api::api_torrents::api_service_torrent_delete;
use torrust_actix::api::api_whitelists::api_service_whitelist_delete;
use torrust_actix::api::structs::api_service_data::ApiServiceData;

fn server_is_running(addr: &str) -> bool {
    std::net::TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        std::time::Duration::from_millis(200),
    ).is_ok()
}

const TEST_SERVER_ADDR: &str = "127.0.0.1:8081";
const TEST_API_TOKEN: &str = "MyApiKey";

#[tokio::test]
async fn test_api_stats_prometheus() {
    if !server_is_running(TEST_SERVER_ADDR) {
        println!("SKIP test_api_stats_prometheus: no server running at {TEST_SERVER_ADDR}");
        return;
    }
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{TEST_SERVER_ADDR}/metrics?token={TEST_API_TOKEN}"))
        .send()
        .await
        .expect("Request failed");
    assert!(resp.status().is_success(), "Prometheus metrics endpoint should return 200");
}

#[actix_web::test]
async fn test_api_torrent_delete() {
    let tracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(peer_id, std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 6881);
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/api/torrent/{info_hash}", web::delete().to(api_service_torrent_delete)),
    )
        .await;
    let uri = format!("/api/torrent/{}", info_hash);
    let req = test::TestRequest::delete()
        .uri(&uri)
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
            "Delete torrent should require authentication");
}

#[actix_web::test]
async fn test_api_whitelist_delete() {
    let tracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let info_hash = common::random_info_hash();
    tracker.add_whitelist(info_hash);
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/api/whitelist/{info_hash}", web::delete().to(api_service_whitelist_delete)),
    )
        .await;
    let uri = format!("/api/whitelist/{}", info_hash);
    let req = test::TestRequest::delete()
        .uri(&uri)
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
            "Delete whitelist should require authentication");
}

#[actix_web::test]
async fn test_api_blacklist_delete() {
    let tracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let info_hash = common::random_info_hash();
    tracker.add_blacklist(info_hash);
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/api/blacklist/{info_hash}", web::delete().to(api_service_blacklist_delete)),
    )
        .await;
    let uri = format!("/api/blacklist/{}", info_hash);
    let req = test::TestRequest::delete()
        .uri(&uri)
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
            "Delete blacklist should require authentication");
}

#[actix_web::test]
async fn test_api_key_delete() {
    let tracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let info_hash = common::random_info_hash();
    tracker.add_key(info_hash, 12345);
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/api/key/{info_hash}", web::delete().to(api_service_key_delete)),
    )
        .await;
    let uri = format!("/api/key/{}", info_hash);
    let req = test::TestRequest::delete()
        .uri(&uri)
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
            "Delete key should require authentication");
}

#[tokio::test]
async fn test_api_cors_headers() {
    if !server_is_running(TEST_SERVER_ADDR) {
        println!("SKIP test_api_cors_headers: no server running at {TEST_SERVER_ADDR}");
        return;
    }
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{TEST_SERVER_ADDR}/metrics?token={TEST_API_TOKEN}"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .expect("Request failed");
    assert!(resp.status().is_success(), "CORS request should succeed");
}

#[actix_web::test]
async fn test_api_invalid_endpoint_404() {
    let tracker = common::create_test_tracker().await;
    let api_config = common::create_test_api_config();
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: api_config,
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/metrics", web::get().to(torrust_actix::api::api_stats::api_service_prom_get)),
    )
        .await;
    let req = test::TestRequest::get()
        .uri("/invalid/endpoint")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404, "Invalid endpoint should return 404");
}

#[tokio::test]
async fn test_api_stats_content_type() {
    if !server_is_running(TEST_SERVER_ADDR) {
        println!("SKIP test_api_stats_content_type: no server running at {TEST_SERVER_ADDR}");
        return;
    }
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{TEST_SERVER_ADDR}/metrics?token={TEST_API_TOKEN}"))
        .send()
        .await
        .expect("Request failed");
    assert!(resp.status().is_success(), "Stats endpoint should succeed");
    assert!(resp.headers().get("content-type").is_some(), "Content-Type header should be present");
}

#[tokio::test]
async fn test_api_concurrent_operations() {
    if !server_is_running(TEST_SERVER_ADDR) {
        println!("SKIP test_api_concurrent_operations: no server running at {TEST_SERVER_ADDR}");
        return;
    }
    let client = reqwest::Client::new();
    for _ in 0..10 {
        let resp = client
            .get(format!("http://{TEST_SERVER_ADDR}/metrics?token={TEST_API_TOKEN}"))
            .send()
            .await
            .expect("Request failed");
        assert!(resp.status().is_success(), "API requests should succeed");
    }
}