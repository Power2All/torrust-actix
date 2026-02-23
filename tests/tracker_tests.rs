mod common;
use std::net::{
    IpAddr,
    Ipv4Addr
};
use torrust_actix::common::structs::number_of_bytes::NumberOfBytes;
use torrust_actix::tracker::enums::torrent_peers_type::TorrentPeersType;

#[tokio::test]
async fn test_add_peer_to_new_torrent() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    let (previous, current) = tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    assert!(previous.is_none(), "Should be no previous entry for new torrent");
    assert_eq!(current.peers.len(), 1, "Should have 1 peer");
    assert_eq!(current.seeds.len(), 0, "Should have 0 seeds (left > 0)");
}

#[tokio::test]
async fn test_add_seed_to_torrent() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let mut seed = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    seed.left = NumberOfBytes(0);
    let (_previous, current) = tracker.add_torrent_peer(info_hash, peer_id, seed, false);
    assert_eq!(current.seeds.len(), 1, "Should have 1 seed");
    assert_eq!(current.peers.len(), 0, "Should have 0 peers");
}

#[tokio::test]
async fn test_peer_to_seed_transition() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    let mut seed = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    seed.left = NumberOfBytes(0);
    let (previous, current) = tracker.add_torrent_peer(info_hash, peer_id, seed, true);
    assert!(previous.is_some(), "Should have previous entry");
    assert_eq!(previous.unwrap().peers.len(), 1, "Previous should have 1 peer");
    assert_eq!(current.seeds.len(), 1, "Current should have 1 seed");
    assert_eq!(current.peers.len(), 0, "Current should have 0 peers");
    assert_eq!(current.completed, 1, "Completed count should increment");
}

#[tokio::test]
async fn test_remove_peer_from_torrent() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    let (previous, current) = tracker.remove_torrent_peer(info_hash, peer_id, false, false);
    assert!(previous.is_some(), "Should have previous entry");
    assert!(current.is_none(), "Torrent should be removed when empty (non-persistent)");
}

#[tokio::test]
async fn test_get_peers_with_limit() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    for i in 0..10 {
        let peer_id = common::random_peer_id();
        let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8)), 6881);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }
    let result = tracker.get_torrent_peers(info_hash, 5, TorrentPeersType::IPv4, None);
    assert!(result.is_some(), "Should return peers");
    let peers = result.unwrap();
    assert_eq!(peers.peers_ipv4.len(), 5, "Should return exactly 5 peers (early exit optimization)");
}

#[tokio::test]
async fn test_get_peers_ipv4_filtering() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id_v4 = common::random_peer_id();
    let peer_v4 = common::create_test_peer(peer_id_v4, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 6881);
    tracker.add_torrent_peer(info_hash, peer_id_v4, peer_v4, false);
    let peer_id_v6 = common::random_peer_id();
    let peer_v6 = common::create_test_peer(peer_id_v6, IpAddr::V6("::1".parse().unwrap()), 6881);
    tracker.add_torrent_peer(info_hash, peer_id_v6, peer_v6, false);
    let result = tracker.get_torrent_peers(info_hash, 0, TorrentPeersType::IPv4, None);
    assert!(result.is_some());
    let peers = result.unwrap();
    assert_eq!(peers.peers_ipv4.len(), 1, "Should have 1 IPv4 peer");
    assert_eq!(peers.peers_ipv6.len(), 0, "Should have 0 IPv6 peers");
}

#[tokio::test]
async fn test_torrent_sharding_distribution() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    for _ in 0..256 {
        let info_hash = common::random_info_hash();
        let peer_id = common::random_peer_id();
        let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }
    let total_torrents = tracker.torrents_sharding.get_torrents_amount();
    assert_eq!(total_torrents, 256, "Should have 256 torrents across shards");
}

#[tokio::test]
async fn test_concurrent_peer_additions() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let mut handles = vec![];
    for i in 0..100 {
        let tracker_clone = tracker.clone();
        let handle = tokio::spawn(async move {
            let peer_id = common::random_peer_id();
            let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8)), 6881);
            tracker_clone.add_torrent_peer(info_hash, peer_id, peer, false);
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.await.expect("Task should complete");
    }
    let result = tracker.get_torrent_peers(info_hash, 0, TorrentPeersType::All, None);
    assert!(result.is_some());
    let peers = result.unwrap();
    assert_eq!(
        peers.peers_ipv4.len() + peers.peers_ipv6.len(),
        100,
        "Should have 100 peers after concurrent additions"
    );
}

#[tokio::test]
async fn test_stats_tracking() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let initial_stats = tracker.get_stats();
    let initial_torrents = initial_stats.torrents;
    let initial_peers = initial_stats.peers;
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    let updated_stats = tracker.get_stats();
    assert_eq!(updated_stats.torrents, initial_torrents + 1, "Torrent count should increment");
    assert_eq!(updated_stats.peers, initial_peers + 1, "Peer count should increment");
}

#[tokio::test]
async fn test_whitelist_filtering() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    tracker.add_whitelist(info_hash);
    let is_whitelisted = tracker.check_whitelist(info_hash);
    assert!(is_whitelisted, "InfoHash should be whitelisted");
}

#[tokio::test]
async fn test_blacklist_filtering() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    tracker.add_blacklist(info_hash);
    let is_blacklisted = tracker.check_blacklist(info_hash);
    assert!(is_blacklisted, "InfoHash should be blacklisted");
}

#[tokio::test]
async fn test_rtc_store_and_take_pending_answers() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let sdp_answer = "v=0\r\no=- 2 2 IN IP4 127.0.0.1\r\ns=answer\r\n".to_string();
    let answers = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert!(answers.is_empty(), "Should start with no pending answers");
    let seeder_peer = common::create_rtc_peer(
        seeder_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        6881,
        Some("v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=offer\r\n".to_string()),
        0,
    );
    tracker.add_torrent_peer(info_hash, seeder_id, seeder_peer, false);
    let stored = tracker.store_rtc_answer(info_hash, seeder_id, leecher_id, sdp_answer.clone());
    assert!(stored, "store_rtc_answer should succeed when seeder exists");
    let answers = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert_eq!(answers.len(), 1, "Should have exactly 1 pending answer");
    assert_eq!(answers[0].0, leecher_id, "Answer should be from the leecher");
    assert_eq!(answers[0].1, sdp_answer, "Answer SDP should match");
    let answers_again = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert!(answers_again.is_empty(), "Answers should be consumed after take");
}

#[tokio::test]
async fn test_rtc_multiple_answers_queued() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let seeder_peer = common::create_rtc_peer(
        seeder_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        6881,
        Some("v=0\r\ns=offer\r\n".to_string()),
        0,
    );
    tracker.add_torrent_peer(info_hash, seeder_id, seeder_peer, false);
    for i in 0..3u8 {
        let leecher_id = common::random_peer_id();
        tracker.store_rtc_answer(
            info_hash,
            seeder_id,
            leecher_id,
            format!("v=0\r\ns=answer{}\r\n", i),
        );
    }
    let answers = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert_eq!(answers.len(), 3, "Should have 3 queued answers");
}

#[tokio::test]
async fn test_rtc_pending_answers_survive_re_announce() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let sdp_answer = "v=0\r\ns=answer\r\n".to_string();
    let seeder_peer = common::create_rtc_peer(
        seeder_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        6881,
        Some("v=0\r\ns=offer\r\n".to_string()),
        0,
    );
    tracker.add_torrent_peer(info_hash, seeder_id, seeder_peer.clone(), false);
    tracker.store_rtc_answer(info_hash, seeder_id, leecher_id, sdp_answer.clone());
    tracker.add_torrent_peer(info_hash, seeder_id, seeder_peer, false);
    let answers = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert_eq!(answers.len(), 1, "Pending answer should survive seeder re-announce");
    assert_eq!(answers[0].1, sdp_answer);
}

#[tokio::test]
async fn test_rtc_update_sdp_offer() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let offer = "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=offer\r\n".to_string();
    let peer = common::create_rtc_peer(
        peer_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        6881,
        None,
        0,
    );
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    let updated = tracker.update_rtc_sdp_offer(info_hash, peer_id, offer.clone());
    assert!(updated, "update_rtc_sdp_offer should succeed when peer exists");
    let entry = tracker.get_torrent(info_hash).expect("Torrent should exist");
    let stored_peer = entry.rtc_seeds.get(&peer_id).expect("Seeder should be in rtc_seeds");
    assert_eq!(
        stored_peer.rtc_sdp_offer.as_deref(),
        Some(offer.as_str()),
        "Stored SDP offer should match"
    );
}

#[tokio::test]
async fn test_rtc_get_peers_leecher_sees_seeders() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let offer = "v=0\r\ns=offer\r\n".to_string();
    let seeder_peer = common::create_rtc_peer(
        seeder_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        6881,
        Some(offer.clone()),
        0,
    );
    tracker.add_torrent_peer(info_hash, seeder_id, seeder_peer, false);
    tracker.update_rtc_sdp_offer(info_hash, seeder_id, offer.clone());
    let entry = tracker.get_rtctorrent_peers(info_hash, false, leecher_id);
    assert!(!entry.rtc_seeds.is_empty(), "Leecher should see at least one seeder");
    let peer = entry.rtc_seeds.get(&seeder_id).expect("Seeder should appear in rtc_seeds");
    assert_eq!(peer.rtc_sdp_offer.as_deref(), Some(offer.as_str()));
}

#[tokio::test]
async fn test_rtc_get_peers_seeder_excluded_from_own_list() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let seeder_peer = common::create_rtc_peer(
        seeder_id,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        6881,
        Some("v=0\r\ns=offer\r\n".to_string()),
        0,
    );
    tracker.add_torrent_peer(info_hash, seeder_id, seeder_peer, false);
    tracker.update_rtc_sdp_offer(info_hash, seeder_id, "v=0\r\ns=offer\r\n".to_string());
    let entry = tracker.get_rtctorrent_peers(info_hash, true, seeder_id);
    assert!(entry.rtc_peers.is_empty(), "No leechers should be present");
}

#[tokio::test]
async fn test_rtc_store_answer_for_nonexistent_peer_returns_false() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let leecher_id = common::random_peer_id();
    let stored = tracker.store_rtc_answer(
        info_hash,
        seeder_id,
        leecher_id,
        "v=0\r\ns=answer\r\n".to_string(),
    );
    assert!(!stored, "store_rtc_answer should fail when seeder does not exist");
}