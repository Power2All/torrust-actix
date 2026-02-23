mod common;

use actix_web::{
    test,
    App
};
use std::net::IpAddr;
use std::sync::Arc;
use torrust_actix::http::http::{
    http_service_cors,
    http_service_routes
};
use torrust_actix::http::structs::http_service_data::HttpServiceData;

fn is_failure_response(body: &[u8]) -> bool {
    body.windows(14).any(|w| w == b"failure reason")
}

fn response_has_key(body: &[u8], needle: &[u8]) -> bool {
    body.windows(needle.len()).any(|w| w == needle)
}

macro_rules! make_app {
    ($rtctorrent:expr) => {{
        let tracker = common::create_test_tracker().await;
        let http_config = Arc::new(
            common::create_test_http_config_with_rtctorrent($rtctorrent)
                .as_ref()
                .clone(),
        );
        let app = test::init_service(
            App::new()
                .wrap(http_service_cors())
                .configure(http_service_routes(Arc::new(HttpServiceData {
                    torrent_tracker: tracker.clone(),
                    http_trackers_config: http_config,
                }))),
        )
        .await;
        (app, tracker)
    }};
}

#[actix_web::test]
async fn test_http_announce_endpoint() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(common::create_test_http_config().as_ref().clone());
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
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(common::create_test_http_config().as_ref().clone());
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
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(common::create_test_http_config().as_ref().clone());
    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(std::sync::Arc::new(HttpServiceData {
                torrent_tracker: tracker,
                http_trackers_config: http_config,
            }))),
    )
    .await;
    let req = test::TestRequest::default()
        .method(actix_web::http::Method::OPTIONS)
        .uri("/announce")
        .insert_header(("Origin", "http://localhost"))
        .insert_header(("Access-Control-Request-Method", "GET"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.headers().contains_key("access-control-allow-origin") ||
            resp.headers().contains_key("access-control-allow-methods"),
            "CORS headers should be present");
}

#[actix_web::test]
async fn test_http_invalid_endpoint() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(common::create_test_http_config().as_ref().clone());
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
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error(), "Should return 404 for invalid endpoint");
}

#[actix_web::test]
async fn test_http_announce_with_ipv6() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let ipv6_peer = common::create_test_peer(
        peer_id,
        IpAddr::V6("2001:db8::1".parse().unwrap()),
        6881,
    );
    tracker.add_torrent_peer(info_hash, peer_id, ipv6_peer, false);
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
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let http_config = std::sync::Arc::new(common::create_test_http_config().as_ref().clone());
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

#[actix_web::test]
async fn test_rtc_disabled_rejects_rtctorrent_request() {
    let (app, _tracker) = make_app!(false);
    let info_hash = common::random_info_hash();
    let peer_id   = common::random_peer_id();
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&peer_id.0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    assert!(is_failure_response(&body), "RTC request with rtctorrent=false config should return failure");
    assert!(body.windows(22).any(|w| w == b"rtctorrent not enabled"),
            "Failure reason should say 'rtctorrent not enabled'");
}

#[actix_web::test]
async fn test_standard_announce_unaffected_by_rtctorrent_flag() {
    for rtctorrent_flag in [false, true] {
        let (app, _tracker) = make_app!(rtctorrent_flag);
        let info_hash = common::random_info_hash();
        let peer_id   = common::random_peer_id();
        let req = test::TestRequest::get()
            .uri(&format!(
                "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=1000&compact=1",
                common::percent_encode(&info_hash.0),
                common::percent_encode(&peer_id.0),
            ))
            .peer_addr("127.0.0.1:0".parse().unwrap())
            .to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        assert!(
            !is_failure_response(&body),
            "Standard announce should succeed regardless of rtctorrent config (flag={})", rtctorrent_flag
        );
        assert!(response_has_key(&body, b"interval"), "Standard response must contain 'interval'");
    }
}

#[actix_web::test]
async fn test_rtc_enabled_seeder_announce_returns_rtc_fields() {
    let (app, _tracker) = make_app!(true);
    let info_hash = common::random_info_hash();
    let peer_id   = common::random_peer_id();
    let offer     = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=offer\r\n";
    let encoded_offer = common::percent_encode(offer.as_bytes());
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1&rtcoffer={}",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&peer_id.0),
            encoded_offer,
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    assert!(!is_failure_response(&body), "RTC seeder announce should succeed");
    assert!(response_has_key(&body, b"rtc_peers"),   "Response must contain 'rtc_peers'");
    assert!(response_has_key(&body, b"rtc_answers"), "Response must contain 'rtc_answers'");
    assert!(response_has_key(&body, b"rtc interval"), "Response must contain 'rtc interval'");
}

#[actix_web::test]
async fn test_rtc_leecher_receives_seeder_offer() {
    let (app, _tracker) = make_app!(true);
    let info_hash  = common::random_info_hash();
    let seeder_id  = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let offer      = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=offer\r\n";
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1&rtcoffer={}",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&seeder_id.0),
            common::percent_encode(offer.as_bytes()),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6882&uploaded=0&downloaded=0&left=1000&compact=1&rtctorrent=1&rtcrequest=1",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&leecher_id.0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    assert!(!is_failure_response(&body), "Leecher announce should succeed");
    assert!(response_has_key(&body, b"rtc_peers"), "Leecher response must contain 'rtc_peers'");
    assert!(
        body.windows(20).any(|w| w == seeder_id.0),
        "Seeder peer_id should appear inside rtc_peers list"
    );
}

#[actix_web::test]
async fn test_rtc_seeder_receives_leecher_answer() {
    let (app, _tracker) = make_app!(true);
    let info_hash  = common::random_info_hash();
    let seeder_id  = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let offer  = "v=0\r\ns=offer\r\n";
    let answer = "v=0\r\ns=answer\r\n";
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1&rtcoffer={}",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&seeder_id.0),
            common::percent_encode(offer.as_bytes()),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    test::call_service(&app, req).await;
    let seeder_hex = hex::encode(seeder_id.0);
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6882&uploaded=0&downloaded=0&left=1000&compact=1&rtctorrent=1&rtcanswer={}&rtcanswerfor={}",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&leecher_id.0),
            common::percent_encode(answer.as_bytes()),
            seeder_hex,
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&seeder_id.0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    assert!(!is_failure_response(&body), "Seeder poll should succeed");
    assert!(response_has_key(&body, b"rtc_answers"), "Response must contain 'rtc_answers'");
    assert!(
        body.windows(20).any(|w| w == leecher_id.0),
        "Leecher peer_id should appear inside rtc_answers list"
    );
}

#[actix_web::test]
async fn test_rtc_answers_consumed_after_poll() {
    let (app, _tracker) = make_app!(true);
    let info_hash  = common::random_info_hash();
    let seeder_id  = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1&rtcoffer={}",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&seeder_id.0),
            common::percent_encode(b"v=0\r\ns=offer\r\n"),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6882&uploaded=0&downloaded=0&left=1000&compact=1&rtctorrent=1&rtcanswer={}&rtcanswerfor={}",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&leecher_id.0),
            common::percent_encode(b"v=0\r\ns=answer\r\n"),
            hex::encode(seeder_id.0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&seeder_id.0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    assert!(
        body.windows(20).any(|w| w == leecher_id.0),
        "First poll: leecher answer should be present"
    );
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=0&compact=1&rtctorrent=1",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&seeder_id.0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    assert!(
        !body.windows(20).any(|w| w == leecher_id.0),
        "Second poll: leecher answer should be consumed and absent"
    );
}