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
async fn scrape_without_a_key_is_refused_when_keys_are_enabled() {
    let config = Arc::new(common::build_test_config(|config| {
        config.tracker_config.keys_enabled = true;
    }));
    let tracker = Arc::new(
        torrust_actix::tracker::structs::torrent_tracker::TorrentTracker::new(config, false).await,
    );
    let http_config = Arc::new(common::create_test_http_config().as_ref().clone());
    let app = test::init_service(
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(Arc::new(HttpServiceData {
                torrent_tracker: tracker.clone(),
                http_trackers_config: http_config,
            }))),
    )
    .await;
    let info_hash = common::random_info_hash();

    // `/announce` already answers "missing key" in this mode; `/scrape` used to report swarm
    // sizes for any info-hash to anyone, which is the whole thing keys exist to prevent.
    let peer: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    let req = test::TestRequest::get()
        .uri(&format!("/scrape?info_hash={}", hex::encode(info_hash.0)))
        .peer_addr(peer)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    assert!(
        response_has_key(&body, b"missing key"),
        "got {:?}",
        String::from_utf8_lossy(&body)
    );

    // The keyed route still reaches key validation rather than being swallowed by the guard.
    let req = test::TestRequest::get()
        .uri(&format!(
            "/{}/scrape?info_hash={}",
            "a".repeat(40),
            hex::encode(info_hash.0)
        ))
        .peer_addr(peer)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    assert!(
        response_has_key(&body, b"unknown key"),
        "got {:?}",
        String::from_utf8_lossy(&body)
    );
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

#[actix_web::test]
async fn test_announce_without_user_key_refused_when_users_enabled() {
    // `/announce`, and `/{key}/announce` with keys enabled, carry no user key and used to be
    // served anyway; UDP already refused them.
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    for keys_enabled in [false, true] {
        let config = Arc::new(common::build_test_config(|c| {
            c.tracker_config.users_enabled = true;
            c.tracker_config.keys_enabled = keys_enabled;
        }));
        let tracker = Arc::new(TorrentTracker::new(config, false).await);
        let access_key = common::random_info_hash();
        tracker.add_key_absolute(access_key, 0);
        let app = test::init_service(App::new().configure(http_service_routes(Arc::new(HttpServiceData {
            torrent_tracker: tracker,
            http_trackers_config: common::create_test_http_config(),
        })))).await;
        let query = format!(
            "announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=1000&compact=1",
            common::percent_encode(&common::random_info_hash().0),
            common::percent_encode(&common::random_peer_id().0),
        );
        let paths = if keys_enabled { vec![format!("/{access_key}/{query}")] } else { vec![format!("/{query}")] };
        for path in paths {
            let req = test::TestRequest::get().uri(&path).peer_addr("127.0.0.1:0".parse().unwrap()).to_request();
            let body = test::read_body(test::call_service(&app, req).await).await;
            assert!(response_has_key(&body, b"unknown user key"), "{path} must be refused without a user key");
        }
    }
}

#[actix_web::test]
async fn test_user_stats_are_credited_over_http() {
    // The HTTP route resolved the key but handed the key itself on as the user id, so the user
    // was never found and nothing was credited. The only peer stopping also drops the torrent,
    // which used to skip the credit as well.
    use std::collections::BTreeMap;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
    use torrust_actix::tracker::structs::user_entry_item::UserEntryItem;
    use torrust_actix::tracker::structs::user_id::UserId;
    let config = Arc::new(common::build_test_config(|c| c.tracker_config.users_enabled = true));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let (user_id, user_key) = (UserId([1; 20]), UserId([0x12; 20]));
    tracker.add_user(user_id, UserEntryItem {
        key: user_key,
        user_id: Some(1),
        user_uuid: None,
        uploaded: 0,
        downloaded: 0,
        completed: 0,
        updated: 0,
        active: 1,
        torrents_active: BTreeMap::new(),
    });
    let app = test::init_service(App::new().configure(http_service_routes(Arc::new(HttpServiceData {
        torrent_tracker: tracker.clone(),
        http_trackers_config: common::create_test_http_config(),
    })))).await;
    let info_hash = common::percent_encode(&common::random_info_hash().0);
    let peer_id = common::percent_encode(&common::random_peer_id().0);
    for (event, uploaded, downloaded) in [("started", 0, 0), ("stopped", 500, 300)] {
        let req = test::TestRequest::get()
            .uri(&format!("/{}/announce?info_hash={info_hash}&peer_id={peer_id}&port=6881&uploaded={uploaded}&downloaded={downloaded}&left=1000&compact=1&event={event}", hex::encode(user_key.0)))
            .peer_addr("127.0.0.1:0".parse().unwrap())
            .to_request();
        let body = test::read_body(test::call_service(&app, req).await).await;
        assert!(!is_failure_response(&body), "{event}: {}", String::from_utf8_lossy(&body));
    }
    let user = tracker.get_user(user_id).unwrap();
    assert_eq!((user.uploaded, user.downloaded), (500, 300));
}

#[actix_web::test]
async fn test_dictionary_peer_list_follows_bep3() {
    // `peer id` is the raw 20 bytes (it went out as 40 hex characters), and `no_peer_id` drops it.
    let (app, tracker) = make_app!(false);
    let info_hash = common::random_info_hash();
    let seed_id = common::random_peer_id();
    let mut seed = common::create_test_peer(seed_id, "10.0.0.7".parse().unwrap(), 6881);
    seed.left = torrust_actix::common::structs::number_of_bytes::NumberOfBytes(0);
    tracker.add_torrent_peer(info_hash, seed_id, seed, false);
    for (extra, expect_id) in [("", true), ("&no_peer_id=1", false)] {
        let req = test::TestRequest::get()
            .uri(&format!(
                "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=1000&compact=0{extra}",
                common::percent_encode(&info_hash.0),
                common::percent_encode(&common::random_peer_id().0),
            ))
            .peer_addr("127.0.0.1:0".parse().unwrap())
            .to_request();
        let body = test::read_body(test::call_service(&app, req).await).await;
        let raw = [b"7:peer id20:".as_slice(), &seed_id.0].concat();
        assert_eq!(response_has_key(&body, &raw), expect_id, "{extra}");
        assert!(!response_has_key(&body, hex::encode(seed_id.0).as_bytes()), "never as hex");
    }
}

#[actix_web::test]
async fn test_rtc_offers_are_capped_by_numwant() {
    let (app, tracker) = make_app!(true);
    let info_hash = common::random_info_hash();
    for i in 0..5u8 {
        let seed_id = common::random_peer_id();
        let seed = common::create_rtc_peer(seed_id, std::net::IpAddr::from([10, 0, 0, i]), 6881, Some(format!("v=0 offer {i}")), 0);
        tracker.add_torrent_peer(info_hash, seed_id, seed, false);
    }
    let req = test::TestRequest::get()
        .uri(&format!(
            "/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=100&compact=1&rtctorrent=1&rtcrequest=1&numwant=2",
            common::percent_encode(&info_hash.0),
            common::percent_encode(&common::random_peer_id().0),
        ))
        .peer_addr("127.0.0.1:0".parse().unwrap())
        .to_request();
    let body = test::read_body(test::call_service(&app, req).await).await;
    assert_eq!(body.windows(9).filter(|w| w == b"sdp_offer").count(), 2);
}
