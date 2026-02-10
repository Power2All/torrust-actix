mod common;

use std::net::IpAddr;
use torrust_actix::config::structs::webtorrent_trackers_config::WebTorrentTrackersConfig;
use torrust_actix::stats::enums::stats_event::StatsEvent;
use torrust_actix::webtorrent::enums::wt_message::WtMessage;
use torrust_actix::webtorrent::enums::wt_message_type::WtMessageType;
use torrust_actix::webtorrent::structs::webtorrent_peer::WebTorrentPeer;
use torrust_actix::webtorrent::structs::wt_announce::WtAnnounce;
use torrust_actix::webtorrent::structs::wt_announce_response::WtAnnounceResponse;
use torrust_actix::webtorrent::structs::wt_answer::WtAnswer;
use torrust_actix::webtorrent::structs::wt_offer::WtOffer;
use torrust_actix::webtorrent::structs::wt_peer_info::WtPeerInfo;
use torrust_actix::webtorrent::structs::wt_scrape::WtScrape;
use torrust_actix::webtorrent::structs::wt_scrape_info::WtScrapeInfo;
use torrust_actix::webtorrent::structs::wt_scrape_response::WtScrapeResponse;
use torrust_actix::webtorrent::webtorrent::{
    handle_webtorrent_announce,
    handle_webtorrent_scrape
};

#[test]
fn test_webtorrent_announce_message_serialization() {
    let announce = WtAnnounce {
        info_hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
        peer_id: "0123456789abcdef01234".to_string(),
        port: 6881,
        uploaded: 1000,
        downloaded: 500,
        left: Some(50000),
        event: Some("start".to_string()),
        numwant: Some(50),
        offer: Some("sdp_offer_data".to_string()),
        answer: None,
        offer_id: None,
        offers_only: None,
    };
    let json = serde_json::to_string(&announce).unwrap();
    assert!(json.contains("\"info_hash\""));
    assert!(json.contains("\"peer_id\""));
    assert!(json.contains("\"uploaded\":1000"));
    let message = WtMessage::Announce(announce);
    let message_json = serde_json::to_string(&message).unwrap();
    assert!(message_json.contains("\"action\":\"announce\""));
}

#[test]
fn test_webtorrent_announce_response_serialization() {
    let response = WtAnnounceResponse {
        info_hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
        complete: 5,
        incomplete: 10,
        peers: vec![
            WtPeerInfo {
                peer_id: "peer123456789012345".to_string(),
                offer: Some("sdp_offer".to_string()),
                offer_id: Some("offer_id_123".to_string()),
                ip: Some("127.0.0.1".to_string()),
                port: Some(6881),
            }
        ],
        interval: 120,
        failure_reason: None,
        warning_message: None,
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"complete\":5"));
    assert!(json.contains("\"incomplete\":10"));
    assert!(json.contains("\"interval\":120"));
}

#[test]
fn test_webtorrent_scrape_serialization() {
    let scrape = WtScrape {
        info_hash: vec![
            "0123456789abcdef0123456789abcdef01234567".to_string(),
            "abcdef0123456789abcdef0123456789abcdef01".to_string(),
        ],
    };
    let json = serde_json::to_string(&scrape).unwrap();
    assert!(json.contains("\"info_hash\""));
    let message = WtMessage::Scrape(scrape);
    let message_json = serde_json::to_string(&message).unwrap();
    assert!(message_json.contains("\"action\":\"scrape\""));
}

#[test]
fn test_webtorrent_scrape_response_serialization() {
    let mut files = std::collections::HashMap::new();
    files.insert("hash123".to_string(), WtScrapeInfo {
        complete: 10,
        downloaded: 100,
        incomplete: 5,
    });
    let response = WtScrapeResponse { files };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"complete\":10"));
    assert!(json.contains("\"downloaded\":100"));
    assert!(json.contains("\"incomplete\":5"));
}

#[test]
fn test_webtorrent_peer_creation() {
    let peer_id = [1u8; 20];
    let addr = "127.0.0.1:6881".parse().unwrap();
    let peer = WebTorrentPeer::new(peer_id, addr);
    assert_eq!(peer.peer_id, peer_id);
    assert_eq!(peer.peer_addr, addr);
    assert_eq!(peer.uploaded, 0);
    assert_eq!(peer.downloaded, 0);
    assert!(peer.offer.is_none());
}

#[test]
fn test_webtorrent_peer_update() {
    let peer_id = [1u8; 20];
    let addr = "127.0.0.1:6881".parse().unwrap();
    let mut peer = WebTorrentPeer::new(peer_id, addr);
    peer.update(1000, 500, 0);
    assert_eq!(peer.uploaded, 1000);
    assert_eq!(peer.downloaded, 500);
    assert_eq!(peer.left, 0);
    assert_eq!(peer.is_seeder, Some(true));
}

#[test]
fn test_webtorrent_peer_offer_handling() {
    let peer_id = [1u8; 20];
    let addr = "127.0.0.1:6881".parse().unwrap();
    let mut peer = WebTorrentPeer::new(peer_id, addr);
    peer.set_offer("sdp_offer_data".to_string(), "offer_id_123".to_string());
    assert_eq!(peer.offer, Some("sdp_offer_data".to_string()));
    assert_eq!(peer.offer_id, Some("offer_id_123".to_string()));
    peer.clear_offer();
    assert!(peer.offer.is_none());
    assert!(peer.offer_id.is_none());
}

#[test]
fn test_webtorrent_peer_timeout() {
    let peer_id = [1u8; 20];
    let addr = "127.0.0.1:6881".parse().unwrap();
    let peer = WebTorrentPeer::new(peer_id, addr);
    assert!(!peer.is_timeout(60));
    assert!(!peer.is_timeout(120));
    assert!(!peer.is_timeout(300));
}

#[test]
fn test_webtorrent_peer_generate_offer_id() {
    let peer_id = [1u8; 20];
    let addr = "127.0.0.1:6881".parse().unwrap();
    let peer = WebTorrentPeer::new(peer_id, addr);
    let offer_id = peer.generate_offer_id();
    assert!(offer_id.len() > 10);
    assert!(offer_id.contains("-"));
}

#[test]
fn test_webtorrent_config_default() {
    let config = WebTorrentTrackersConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.bind_address, "0.0.0.0:12100");
    assert_eq!(config.keep_alive, 60);
    assert_eq!(config.threads, 4);
    assert!(!config.ssl);
}

#[test]
fn test_webtorrent_message_type_detection() {
    let announce = WtAnnounce {
        info_hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
        peer_id: "0123456789abcdef01234".to_string(),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: Some(1000),
        event: None,
        numwant: None,
        offer: None,
        answer: None,
        offer_id: None,
        offers_only: None,
    };
    let message = WtMessage::Announce(announce);
    assert_eq!(message.message_type(), WtMessageType::Announce);
    let scrape = WtScrape {
        info_hash: vec!["hash123".to_string()],
    };
    let message = WtMessage::Scrape(scrape);
    assert_eq!(message.message_type(), WtMessageType::Scrape);
}

#[actix_web::test]
async fn test_webtorrent_announce_handler() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let announce = WtAnnounce {
        info_hash: hex::encode(info_hash.0),
        peer_id: hex::encode(peer_id.0),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: Some(1000),
        event: Some("start".to_string()),
        numwant: Some(10),
        offer: None,
        answer: None,
        offer_id: None,
        offers_only: None,
    };
    let ip = IpAddr::V4("127.0.0.1".parse().unwrap());
    let result: Result<WtAnnounceResponse, _> = handle_webtorrent_announce(&tracker, announce, ip).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.info_hash, hex::encode(info_hash.0));
    assert_eq!(response.interval, tracker.config.tracker_config.request_interval as i64);
}

#[actix_web::test]
async fn test_webtorrent_scrape_handler() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let scrape = WtScrape {
        info_hash: vec![hex::encode(info_hash.0)],
    };
    let result: Result<WtScrapeResponse, _> = handle_webtorrent_scrape(&tracker, scrape).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.files.is_empty());
}

#[test]
fn test_webtorrent_offer_serialization() {
    let offer = WtOffer {
        info_hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
        peer_id: "0123456789abcdef01234".to_string(),
        offer: "sdp_offer_content".to_string(),
        offer_id: "offer_unique_id".to_string(),
    };
    let message = WtMessage::Offer(offer);
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"action\":\"offer\""));
    assert!(json.contains("\"offer_id\""));
    assert!(json.contains("\"sdp_offer_content\""));
}

#[test]
fn test_webtorrent_answer_serialization() {
    let answer = WtAnswer {
        info_hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
        peer_id: "0123456789abcdef01234".to_string(),
        answer: "sdp_answer_content".to_string(),
        offer_id: "offer_unique_id".to_string(),
        to_peer_id: "target_peer_id".to_string(),
    };
    let message = WtMessage::Answer(answer);
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"action\":\"answer\""));
    assert!(json.contains("\"to_peer_id\""));
    assert!(json.contains("\"sdp_answer_content\""));
}

#[test]
fn test_webtorrent_statistics_events() {
    let events = vec![
        StatsEvent::Wt4ConnectionsHandled,
        StatsEvent::Wt4AnnouncesHandled,
        StatsEvent::Wt4OffersHandled,
        StatsEvent::Wt4AnswersHandled,
        StatsEvent::Wt4ScrapesHandled,
        StatsEvent::Wt4Failure,
        StatsEvent::Wt6ConnectionsHandled,
        StatsEvent::Wt6AnnouncesHandled,
        StatsEvent::Wt6OffersHandled,
        StatsEvent::Wt6AnswersHandled,
        StatsEvent::Wt6ScrapesHandled,
        StatsEvent::Wt6Failure,
    ];
    assert_eq!(events.len(), 12);
}