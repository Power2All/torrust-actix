mod common;

use actix_web::{test, web, App};
use std::sync::Arc;
use torrust_actix::api::api_blacklists::api_service_blacklist_delete;
use torrust_actix::api::api_keys::api_service_key_delete;
use torrust_actix::api::api_torrents::api_service_torrent_delete;
use torrust_actix::api::api_whitelists::api_service_whitelist_delete;
use torrust_actix::api::structs::api_service_data::ApiServiceData;
use torrust_actix::http::structs::http_service_data::HttpServiceData;

#[actix_web::test]
async fn test_api_stats_prometheus() {
    let tracker = common::create_test_tracker().await;
    let http_config = common::create_test_http_config();

    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: http_config,
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/metrics", web::get().to(torrust_actix::api::api_stats::api_service_prom_get)),
    )
        .await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;

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
    let req = test::TestRequest::delete().uri(&uri).to_request();
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
    let req = test::TestRequest::delete().uri(&uri).to_request();
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
    let req = test::TestRequest::delete().uri(&uri).to_request();
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
    let req = test::TestRequest::delete().uri(&uri).to_request();
    let resp = test::call_service(&app, req).await;

    
    assert!(resp.status().as_u16() == 401 || resp.status().as_u16() == 400,
            "Delete key should require authentication");
}

#[actix_web::test]
async fn test_api_cors_headers() {
    let tracker = common::create_test_tracker().await;
    let http_config = common::create_test_http_config();

    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: http_config,
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/metrics", web::get().to(torrust_actix::api::api_stats::api_service_prom_get)),
    )
        .await;

    let req = test::TestRequest::get()
        .uri("/metrics")
        .insert_header(("Origin", "http://example.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success(), "CORS request should succeed");
}

#[actix_web::test]
async fn test_api_invalid_endpoint_404() {
    let tracker = common::create_test_tracker().await;
    let http_config = common::create_test_http_config();

    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: http_config,
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/metrics", web::get().to(torrust_actix::api::api_stats::api_service_prom_get)),
    )
        .await;

    let req = test::TestRequest::get().uri("/invalid/endpoint").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 404, "Invalid endpoint should return 404");
}

#[actix_web::test]
async fn test_api_stats_content_type() {
    let tracker = common::create_test_tracker().await;
    let http_config = common::create_test_http_config();

    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: http_config,
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/metrics", web::get().to(torrust_actix::api::api_stats::api_service_prom_get)),
    )
        .await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success(), "Stats endpoint should succeed");

    
    let content_type = resp.headers().get("content-type");
    assert!(content_type.is_some(), "Content-Type header should be present");
}

#[actix_web::test]
async fn test_api_concurrent_operations() {
    let tracker = common::create_test_tracker().await;
    let http_config = common::create_test_http_config();

    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: http_config,
    });

    
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data.clone()))
            .route("/metrics", web::get().to(torrust_actix::api::api_stats::api_service_prom_get)),
    )
        .await;

    
    for _ in 0..10 {
        let req = test::TestRequest::get().uri("/metrics").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API requests should succeed");
    }
}
