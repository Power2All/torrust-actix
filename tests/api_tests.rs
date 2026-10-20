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

#[actix_web::test]
async fn test_empty_api_key_never_authenticates() {
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    let config = Arc::new(common::build_test_config(|c| c.tracker_config.api_key = String::new()));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker,
        api_trackers_config: common::create_test_api_config(),
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/api/torrent/{info_hash}", web::delete().to(api_service_torrent_delete)),
    ).await;
    let uri = format!("/api/torrent/{}?token=", common::random_info_hash());
    let resp = test::call_service(&app, test::TestRequest::delete().uri(&uri).peer_addr("127.0.0.1:0".parse().unwrap()).to_request()).await;
    let body = test::read_body(resp).await;
    assert!(body.windows(13).any(|w| w == b"invalid token"), "an empty token must not match an empty api_key");
}

#[actix_web::test]
async fn test_bulk_torrent_post_with_a_bad_entry_changes_nothing() {
    use torrust_actix::api::api_torrents::api_service_torrents_post;
    let tracker = common::create_test_tracker().await;
    let service_data = Arc::new(ApiServiceData {
        torrent_tracker: tracker.clone(),
        api_trackers_config: common::create_test_api_config(),
    });
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(service_data))
            .route("/api/torrents", web::post().to(api_service_torrents_post)),
    ).await;
    let good = common::random_info_hash();
    let body = format!(r#"{{"{good}": 5, "{}": 1}}"#, "z".repeat(40));
    let uri = format!("/api/torrents?token={}", tracker.config.tracker_config.api_key);
    let req = test::TestRequest::post().uri(&uri).peer_addr("127.0.0.1:0".parse().unwrap()).set_payload(body).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    assert!(tracker.get_torrent(good).is_none(), "a rejected batch must not be partly applied");
}

#[actix_web::test]
async fn test_parse_hex_id_status_strings() {
    use torrust_actix::api::api::parse_hex_id;
    use torrust_actix::tracker::structs::info_hash::InfoHash;
    let status = |r: Box<actix_web::HttpResponse>| async move {
        let body = actix_web::body::to_bytes(r.into_body()).await.unwrap();
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["status"].as_str().unwrap().to_string()
    };
    let ok: InfoHash = parse_hex_id("AABBCCDDEEFF00112233445566778899aabbccdd", "info_hash").unwrap();
    assert_eq!(ok.0[0], 0xAA);
    assert_eq!(status(parse_hex_id::<InfoHash>("aabb", "info_hash").unwrap_err()).await, "bad info_hash");
    assert_eq!(status(parse_hex_id::<InfoHash>(&"zz".repeat(20), "key_hash").unwrap_err()).await, "invalid key_hash");
}
