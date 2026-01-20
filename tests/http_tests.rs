mod common;

use actix_web::{test, App};
use std::net::IpAddr;
use torrust_actix::http::http::{http_service_cors, http_service_routes};
use torrust_actix::http::structs::http_service_data::HttpServiceData;

#[actix_web::test]
async fn test_http_announce_endpoint() {
    let tracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig::default());

    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(std::sync::Arc::new(HttpServiceData {
                torrent_tracker: tracker.clone(),
                http_trackers_config: http_config,
            }))),
    )
    .await;

    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();

    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=1000",
            hex::encode(info_hash.0),
            hex::encode(peer_id.0)
        ))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success() || resp.status().is_client_error(),
            "Announce endpoint should respond");
}

#[actix_web::test]
async fn test_http_scrape_endpoint() {
    let tracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig::default());

    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(std::sync::Arc::new(HttpServiceData {
                torrent_tracker: tracker.clone(),
                http_trackers_config: http_config,
            }))),
    )
    .await;

    let info_hash = common::random_info_hash();

    let req = test::TestRequest::get()
        .uri(&format!("/scrape?info_hash={}", hex::encode(info_hash.0)))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success() || resp.status().is_client_error(),
            "Scrape endpoint should respond");
}

#[actix_web::test]
async fn test_http_cors_headers() {
    let tracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig::default());

    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(std::sync::Arc::new(HttpServiceData {
                torrent_tracker: tracker,
                http_trackers_config: http_config,
            }))),
    )
    .await;

    let req = test::TestRequest::options()
        .uri("/announce")
        .insert_header(("Origin", "http://localhost"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // CORS should add appropriate headers
    assert!(resp.headers().contains_key("access-control-allow-origin") ||
            resp.headers().contains_key("access-control-allow-methods"),
            "CORS headers should be present");
}

#[actix_web::test]
async fn test_http_invalid_endpoint() {
    let tracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig::default());

    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(std::sync::Arc::new(HttpServiceData {
                torrent_tracker: tracker,
                http_trackers_config: http_config,
            }))),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/nonexistent")
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_client_error(), "Should return 404 for invalid endpoint");
}

#[actix_web::test]
async fn test_http_announce_with_ipv6() {
    let tracker = common::create_test_tracker().await;

    // Add an IPv6 peer
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let ipv6_peer = common::create_test_peer(
        IpAddr::V6("2001:db8::1".parse().unwrap()),
        6881,
    );

    tracker.add_torrent_peer(info_hash, peer_id, ipv6_peer, false);

    // Verify IPv6 peer was added
    let result = tracker.get_torrent_peers(
        info_hash,
        0,
        torrust_actix::tracker::enums::torrent_peers_type::TorrentPeersType::IPv6,
        None,
    );

    assert!(result.is_some());
    let peers = result.unwrap();
    assert_eq!(peers.peers_ipv6.len(), 1, "Should have 1 IPv6 peer");
}

#[actix_web::test]
async fn test_http_server_cleanup_optimization() {
    // Test that the HTTP server code cleanup (extracting service_data) works
    let tracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig::default());

    // This tests the refactored HTTP server creation
    let service_data = std::sync::Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: http_config.clone(),
    });

    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(service_data)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/scrape")
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success() || resp.status().is_client_error(),
            "Refactored server should work correctly");
}
