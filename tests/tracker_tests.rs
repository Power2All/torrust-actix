mod common;
use std::net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr,
};
use std::time::Duration;
use torrust_actix::common::structs::number_of_bytes::NumberOfBytes;
use torrust_actix::tracker::enums::torrent_peers_type::TorrentPeersType;
use torrust_actix::tracker::structs::torrent_sharding::TorrentSharding;

#[tokio::test]
async fn test_add_peer_to_new_torrent() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    assert!(tracker.get_torrent(info_hash).is_none(), "Should be no previous entry for new torrent");
    let current = tracker.add_torrent_peer(info_hash, peer_id, peer, false);
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
    let current = tracker.add_torrent_peer(info_hash, peer_id, seed, false);
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
    let current = tracker.add_torrent_peer(info_hash, peer_id, seed, true);
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
    let (existed, current) = tracker.remove_torrent_peer(info_hash, peer_id, false, false);
    assert!(existed, "Should have previous entry");
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
    let stored = tracker.store_rtc_answer(info_hash, seeder_id, leecher_id, &sdp_answer);
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
            &format!("v=0\r\ns=answer{}\r\n", i),
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
    tracker.store_rtc_answer(info_hash, seeder_id, leecher_id, &sdp_answer);
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
    let updated = tracker.update_rtc_sdp_offer(info_hash, peer_id, &offer);
    assert!(updated, "update_rtc_sdp_offer should succeed when peer exists");
    let entry = tracker.get_torrent(info_hash).expect("Torrent should exist");
    let stored_peer = entry.rtc_seeds.get(&peer_id).expect("Seeder should be in rtc_seeds");
    assert_eq!(
        stored_peer.rtc_sdp_offer().as_deref(),
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
    tracker.update_rtc_sdp_offer(info_hash, seeder_id, &offer);
    let entry = tracker.get_rtctorrent_peers(info_hash, false, leecher_id);
    assert!(!entry.rtc_seeds.is_empty(), "Leecher should see at least one seeder");
    let peer = entry.rtc_seeds.get(&seeder_id).expect("Seeder should appear in rtc_seeds");
    assert_eq!(peer.rtc_sdp_offer().as_deref(), Some(offer.as_str()));
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
    tracker.update_rtc_sdp_offer(info_hash, seeder_id, "v=0\r\ns=offer\r\n");
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
        "v=0\r\ns=answer\r\n",
    );
    assert!(!stored, "store_rtc_answer should fail when seeder does not exist");
}

// --- Peer reaper tests ---

const TIMEOUT: Duration = Duration::from_secs(300);
const AGED: Duration = Duration::from_secs(600); // older than TIMEOUT → should be reaped

#[tokio::test]
async fn test_reaper_removes_expired_ipv4_peer() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_peer(peer_id, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 6881, AGED), false);
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let entry = tracker.get_torrent(info_hash);
    assert!(entry.is_none(), "Torrent should be gone after all IPv4 peers expire (non-persistent)");
}

#[tokio::test]
async fn test_reaper_removes_expired_ipv6_peer() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_peer(peer_id, IpAddr::V6(Ipv6Addr::new(0x20, 1, 0, 0, 0, 0, 0, 1)), 6881, AGED), false);
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let entry = tracker.get_torrent(info_hash);
    assert!(entry.is_none(), "Torrent should be gone after all IPv6 peers expire (non-persistent)");
}

#[tokio::test]
async fn test_reaper_removes_expired_ipv4_seed() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_seed(peer_id, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 6881, AGED), false);
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let entry = tracker.get_torrent(info_hash);
    assert!(entry.is_none(), "Torrent should be gone after all IPv4 seeds expire (non-persistent)");
}

#[tokio::test]
async fn test_reaper_removes_expired_ipv6_seed() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_seed(peer_id, IpAddr::V6(Ipv6Addr::new(0x20, 1, 0, 0, 0, 0, 0, 1)), 6881, AGED), false);
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let entry = tracker.get_torrent(info_hash);
    assert!(entry.is_none(), "Torrent should be gone after all IPv6 seeds expire (non-persistent)");
}

#[tokio::test]
async fn test_reaper_stats_decremented_for_ipv4_peer() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_peer(peer_id, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 6881, AGED), false);
    let before = tracker.get_stats();
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let after = tracker.get_stats();
    assert_eq!(after.peers, before.peers - 1, "Peers stat should decrement by 1 after IPv4 peer reap");
    assert_eq!(after.torrents, before.torrents - 1, "Torrents stat should decrement after torrent removed");
}

#[tokio::test]
async fn test_reaper_stats_decremented_for_ipv6_peer() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_peer(peer_id, IpAddr::V6(Ipv6Addr::new(0x20, 1, 0, 0, 0, 0, 0, 1)), 6881, AGED), false);
    let before = tracker.get_stats();
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let after = tracker.get_stats();
    assert_eq!(after.peers, before.peers - 1, "Peers stat should decrement by 1 after IPv6 peer reap");
    assert_eq!(after.torrents, before.torrents - 1, "Torrents stat should decrement after torrent removed");
}

#[tokio::test]
async fn test_reaper_stats_decremented_for_ipv6_seed() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_seed(peer_id, IpAddr::V6(Ipv6Addr::new(0x20, 1, 0, 0, 0, 0, 0, 1)), 6881, AGED), false);
    let before = tracker.get_stats();
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let after = tracker.get_stats();
    assert_eq!(after.seeds, before.seeds - 1, "Seeds stat should decrement by 1 after IPv6 seed reap");
    assert_eq!(after.torrents, before.torrents - 1, "Torrents stat should decrement after torrent removed");
}

#[tokio::test]
async fn test_reaper_keeps_fresh_peers() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    // Fresh peer — not aged, should survive the reap
    let fresh = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 6881);
    tracker.add_torrent_peer(info_hash, peer_id, fresh, false);
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let entry = tracker.get_torrent(info_hash);
    assert!(entry.is_some(), "Torrent should still exist when peer is still fresh");
    assert_eq!(entry.unwrap().peers.len(), 1, "Fresh IPv4 peer should not be reaped");
}

#[tokio::test]
async fn test_reaper_only_removes_expired_peers_mixed() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let aged_id = common::random_peer_id();
    let fresh_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, aged_id,  common::create_aged_peer(aged_id,  IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 6881, AGED), false);
    tracker.add_torrent_peer(info_hash, fresh_id, common::create_test_peer(fresh_id, IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)), 6882), false);
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, false).await;
    let entry = tracker.get_torrent(info_hash).expect("Torrent should still exist (has a fresh peer)");
    assert_eq!(entry.peers.len(), 1, "Only the fresh peer should remain");
    assert!(!entry.peers.contains_key(&aged_id), "Aged peer should have been removed");
    assert!(entry.peers.contains_key(&fresh_id), "Fresh peer should still be present");
}

#[tokio::test]
async fn test_reaper_persistent_clears_maps_but_keeps_torrent() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_aged_peer(peer_id, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 6881, AGED), false);
    // Use persistent=true so the torrent entry itself is retained even when empty
    TorrentSharding::cleanup_once(tracker.clone(), TIMEOUT, TIMEOUT, true).await;
    let entry = tracker.get_torrent(info_hash);
    assert!(entry.is_some(), "Torrent entry should be kept in persistent mode");
    let entry = entry.unwrap();
    assert!(entry.peers.is_empty(), "Expired peers should be cleared even in persistent mode");
}

#[tokio::test]
async fn test_announce_entry_counts_exact_when_peer_map_capped() {
    use torrust_actix::tracker::structs::announce_entry::SNAPSHOT_PEER_CAP;
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let total = SNAPSHOT_PEER_CAP + 25;
    let mut snapshot = None;
    for i in 0..total {
        let peer_id = common::random_peer_id();
        let peer = common::create_test_peer(
            peer_id,
            IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)),
            6881,
        );
        snapshot = Some(tracker.add_torrent_peer(info_hash, peer_id, peer, false));
    }
    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.counts.peers_ipv4, total, "counts must reflect the full swarm, not the cap");
    assert_eq!(snapshot.counts.total_peers(), total, "total_peers must be exact");
    assert!(snapshot.peers.len() <= SNAPSHOT_PEER_CAP, "returned peer map must be capped");
    assert!(snapshot.peers.len() >= 72, "bounded map must still cover a full response");
}
#[tokio::test]
async fn test_peer_map_is_capped_per_torrent() {
    use std::sync::Arc;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

    // Peer ids are client-chosen, so an uncapped map grows once per id announced.
    const CAP: u64 = 8;
    let config = Arc::new(common::build_test_config(|config| {
        config.tracker_config.max_peers_per_torrent = CAP;
    }));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let info_hash = common::random_info_hash();

    // Seed the torrent so later announces take the "existing torrent" path.
    let first = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, first, common::create_test_peer(first, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881), false);

    for _ in 0..(CAP * 5) {
        let peer_id = common::random_peer_id();
        let peer = common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }

    let entry = tracker.get_torrent(info_hash).expect("torrent should exist");
    assert_eq!(entry.peers.len() as u64, CAP, "leecher map must stay at the cap");

    // The global counter has to track evictions or it drifts.
    let stats = tracker.get_stats();
    assert_eq!(stats.peers, CAP as i64, "peer statistic must match the peers actually held");
}

#[tokio::test]
async fn test_peer_cap_refreshes_existing_peer_without_evicting() {
    use std::sync::Arc;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

    const CAP: u64 = 4;
    let config = Arc::new(common::build_test_config(|config| {
        config.tracker_config.max_peers_per_torrent = CAP;
    }));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let info_hash = common::random_info_hash();

    let peer_ids: Vec<_> = (0..CAP).map(|_| common::random_peer_id()).collect();
    for peer_id in &peer_ids {
        let peer = common::create_test_peer(*peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
        tracker.add_torrent_peer(info_hash, *peer_id, peer, false);
    }
    assert_eq!(tracker.get_torrent(info_hash).unwrap().peers.len() as u64, CAP);

    // A peer already in the map is a refresh, not an insert, so nobody is pushed out.
    let peer = common::create_test_peer(peer_ids[0], IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6882);
    tracker.add_torrent_peer(info_hash, peer_ids[0], peer, false);
    let entry = tracker.get_torrent(info_hash).unwrap();
    assert_eq!(entry.peers.len() as u64, CAP, "a refresh must not change the map size");
    for peer_id in &peer_ids {
        assert!(entry.peers.contains_key(peer_id), "no peer should have been evicted");
    }
    assert_eq!(tracker.get_stats().peers, CAP as i64);
}

#[tokio::test]
async fn test_rtc_pending_answers_are_capped() {
    use std::sync::Arc;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

    // Filled by *other* peers naming this one, so the cap is what stops anyone making the
    // tracker hold arbitrary SDP on a victim's behalf.
    const CAP: u64 = 4;
    let config = Arc::new(common::build_test_config(|config| {
        config.tracker_config.max_rtc_pending_answers = CAP;
    }));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();

    let mut seeder = common::create_test_peer(seeder_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    seeder.left = NumberOfBytes(0);
    seeder.rtc_data = Some(Box::new(torrust_actix::tracker::structs::rtc_data::RtcData::new(Some("v=0"))));
    tracker.add_torrent_peer(info_hash, seeder_id, seeder, false);

    for _ in 0..(CAP * 10) {
        assert!(tracker.store_rtc_answer(info_hash, seeder_id, common::random_peer_id(), "v=0\r\nanswer"));
    }
    let queued = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert_eq!(queued.len() as u64, CAP, "pending answers must not grow past the cap");

    assert!(tracker.take_rtc_pending_answers(info_hash, seeder_id).is_empty());
}

#[tokio::test]
async fn test_rtc_pending_answer_replaces_same_peer() {
    use std::sync::Arc;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

    let config = Arc::new(common::build_test_config(|config| {
        config.tracker_config.max_rtc_pending_answers = 4;
    }));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let info_hash = common::random_info_hash();
    let seeder_id = common::random_peer_id();
    let answerer_id = common::random_peer_id();

    let mut seeder = common::create_test_peer(seeder_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
    seeder.left = NumberOfBytes(0);
    seeder.rtc_data = Some(Box::new(torrust_actix::tracker::structs::rtc_data::RtcData::new(Some("v=0"))));
    tracker.add_torrent_peer(info_hash, seeder_id, seeder, false);

    tracker.store_rtc_answer(info_hash, seeder_id, answerer_id, "first");
    tracker.store_rtc_answer(info_hash, seeder_id, answerer_id, "second");
    let queued = tracker.take_rtc_pending_answers(info_hash, seeder_id);
    assert_eq!(queued.len(), 1, "the same answerer must occupy one slot");
    assert_eq!(queued[0].1, "second", "the newest answer must win");
}

#[tokio::test]
async fn test_cleanup_survives_timeout_longer_than_uptime() {
    use std::sync::Arc;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

    let config = Arc::new(common::build_test_config(|_| {}));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, peer_id, common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881), false);

    // `Instant` counts from boot on Linux, so a timeout longer than uptime underflows.
    let absurd_timeout = Duration::from_secs(60 * 60 * 24 * 365 * 100);
    TorrentSharding::cleanup_once(tracker.clone(), absurd_timeout, absurd_timeout, false).await;

    let entry = tracker.get_torrent(info_hash).expect("torrent must survive the skipped pass");
    assert_eq!(entry.peers.len(), 1, "no peer should have been removed");
}

#[tokio::test]
async fn test_torrent_updates_dedupe_per_info_hash() {
    use torrust_actix::tracker::enums::updates_action::UpdatesAction;
    use torrust_actix::tracker::structs::torrent_update_data::TorrentUpdateData;

    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();

    // Repeat updates for one torrent collapse, so queue size tracks distinct torrents.
    assert!(tracker.add_torrent_update(info_hash, TorrentUpdateData { seeds_ipv4: 1, ..Default::default() }, UpdatesAction::Add));
    for seeds in 2..50u64 {
        assert!(!tracker.add_torrent_update(info_hash, TorrentUpdateData { seeds_ipv4: seeds, ..Default::default() }, UpdatesAction::Add));
    }
    let pending = tracker.get_torrent_updates();
    assert_eq!(pending.len(), 1, "one torrent must occupy one queue slot");
    assert_eq!(pending[&info_hash].0.seeds_ipv4, 49, "the newest update must win");
    assert_eq!(tracker.get_stats().torrents_updates, 1, "statistic must match the queue length");

    let other = common::random_info_hash();
    assert!(tracker.add_torrent_update(other, TorrentUpdateData::default(), UpdatesAction::Add));
    assert_eq!(tracker.get_torrent_updates().len(), 2, "distinct torrents get distinct slots");
    assert_eq!(tracker.get_stats().torrents_updates, 2);

    tracker.clear_torrent_updates();
    assert!(tracker.get_torrent_updates().is_empty());
    assert_eq!(tracker.get_stats().torrents_updates, 0);
}

#[tokio::test]
async fn test_keys_loaded_from_database_keep_absolute_expiry() {
    use torrust_actix::common::common::current_time;

    let tracker: common::TestTracker = common::create_test_tracker().await;

    // Database rows store an absolute expiry. Feeding one through the relative `add_key` would
    // add it to the current time, pushing a valid key decades out and turning a permanent key
    // (0) into one that expired the instant it loaded.
    let permanent = common::random_info_hash();
    tracker.add_key_absolute(permanent, 0);
    assert_eq!(tracker.get_key(permanent).unwrap().1, 0, "expiry must be stored verbatim");
    assert!(tracker.check_key(permanent), "a stored expiry of 0 means permanent");

    let future = common::random_info_hash();
    let future_expiry = current_time() as i64 + 600;
    tracker.add_key_absolute(future, future_expiry);
    assert_eq!(tracker.get_key(future).unwrap().1, future_expiry);
    assert!(tracker.check_key(future));

    let past = common::random_info_hash();
    tracker.add_key_absolute(past, current_time() as i64 - 600);
    assert!(!tracker.check_key(past), "an already-expired stored key must not validate");

    // The relative form stays relative for callers that pass a duration.
    let relative = common::random_info_hash();
    tracker.add_key(relative, 600);
    assert!(tracker.get_key(relative).unwrap().1 >= current_time() as i64 + 599);
    assert!(tracker.check_key(relative));
}

#[tokio::test]
async fn test_permanent_keys_survive_cleanup() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let permanent = common::random_info_hash();
    let expired = common::random_info_hash();
    tracker.add_key_absolute(permanent, 0);
    tracker.add_key_absolute(expired, 1);

    tracker.clean_keys();

    assert!(tracker.get_key(permanent).is_some(), "permanent keys must not be swept");
    assert!(tracker.get_key(expired).is_none(), "expired keys must be swept");
}

#[tokio::test]
async fn test_rtc_peers_expire_while_bt_cutoff_unavailable() {
    use std::sync::Arc;
    use torrust_actix::tracker::structs::rtc_data::RtcData;
    use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

    // `rtc_peers_timeout` is far shorter than `peers_timeout`, so shortly after boot the BT
    // cutoff underflows while the RTC one is fine. RTC peers must still be swept then.
    let config = Arc::new(common::build_test_config(|_| {}));
    let tracker = Arc::new(TorrentTracker::new(config, false).await);
    let info_hash = common::random_info_hash();

    let bt_peer_id = common::random_peer_id();
    tracker.add_torrent_peer(info_hash, bt_peer_id, common::create_test_peer(bt_peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881), false);

    let rtc_peer_id = common::random_peer_id();
    let mut rtc_peer = common::create_test_peer(rtc_peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 6882);
    rtc_peer.rtc_data = Some(Box::new(RtcData::new(Some("v=0"))));
    tracker.add_torrent_peer(info_hash, rtc_peer_id, rtc_peer, false);

    tokio::time::sleep(Duration::from_millis(20)).await;

    let unavailable_bt_timeout = Duration::from_secs(60 * 60 * 24 * 365 * 100);
    TorrentSharding::cleanup_once(tracker.clone(), unavailable_bt_timeout, Duration::from_millis(1), false).await;

    let entry = tracker.get_torrent(info_hash).expect("torrent must survive");
    assert!(entry.rtc_peers.is_empty(), "RTC peers must expire on their own cutoff");
    assert_eq!(entry.peers.len(), 1, "BT peers must survive an unavailable BT cutoff");
}
