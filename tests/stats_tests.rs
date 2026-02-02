mod common;

use torrust_actix::stats::enums::stats_event::StatsEvent;

#[tokio::test]
async fn test_stats_initial_values() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let stats = tracker.get_stats();
    assert_eq!(stats.torrents, 0, "Initial torrents count should be 0");
    assert_eq!(stats.seeds, 0, "Initial seeds count should be 0");
    assert_eq!(stats.peers, 0, "Initial peers count should be 0");
    assert_eq!(stats.completed, 0, "Initial completed count should be 0");
}

#[tokio::test]
async fn test_stats_increment_decrement() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    tracker.update_stats(StatsEvent::Torrents, 1);
    tracker.update_stats(StatsEvent::Seeds, 5);
    tracker.update_stats(StatsEvent::Peers, 10);
    let stats = tracker.get_stats();
    assert_eq!(stats.torrents, 1, "Torrents should be 1");
    assert_eq!(stats.seeds, 5, "Seeds should be 5");
    assert_eq!(stats.peers, 10, "Peers should be 10");
    tracker.update_stats(StatsEvent::Seeds, -2);
    tracker.update_stats(StatsEvent::Peers, -3);
    let stats = tracker.get_stats();
    assert_eq!(stats.seeds, 3, "Seeds should be 3 after decrement");
    assert_eq!(stats.peers, 7, "Peers should be 7 after decrement");
}

#[tokio::test]
async fn test_stats_concurrent_updates() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let mut handles = vec![];
    for _ in 0..100 {
        let tracker_clone = tracker.clone();
        let handle = tokio::spawn(async move {
            tracker_clone.update_stats(StatsEvent::Peers, 1);
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.await.unwrap();
    }
    let stats = tracker.get_stats();
    assert_eq!(stats.peers, 100, "Peers should be 100 after 100 concurrent increments");
}

#[tokio::test]
async fn test_stats_completed_tracking() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let mut peer = common::create_test_peer(
        peer_id,
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        6881,
    );
    peer.left = torrust_actix::common::structs::number_of_bytes::NumberOfBytes(0);
    tracker.add_torrent_peer(info_hash, peer_id, peer, true);
    let stats = tracker.get_stats();
    assert_eq!(stats.completed, 1, "Completed count should increment");
}

#[tokio::test]
async fn test_stats_seed_peer_ratio() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    for i in 0..5 {
        let peer_id = common::random_peer_id();
        let mut peer = common::create_test_peer(
            peer_id,
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            6881 + i,
        );
        peer.left = torrust_actix::common::structs::number_of_bytes::NumberOfBytes(0);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }
    for i in 0..10 {
        let peer_id = common::random_peer_id();
        let mut peer = common::create_test_peer(
            peer_id,
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
            6881 + i,
        );
        peer.left = torrust_actix::common::structs::number_of_bytes::NumberOfBytes(1000);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }
    let stats = tracker.get_stats();
    assert_eq!(stats.seeds, 5, "Should have 5 seeds");
    assert_eq!(stats.peers, 10, "Should have 10 peers");
}

#[tokio::test]
async fn test_stats_prometheus_format() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    tracker.update_stats(StatsEvent::Torrents, 5);
    tracker.update_stats(StatsEvent::Seeds, 10);
    tracker.update_stats(StatsEvent::Peers, 20);
    let stats = tracker.get_stats();
    assert!(stats.torrents >= 0, "Should have torrents stat");
    assert!(stats.seeds >= 0, "Should have seeds stat");
    assert!(stats.peers >= 0, "Should have peers stat");
}

#[tokio::test]
async fn test_stats_atomic_operations() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let mut handles = vec![];
    for i in 0..50 {
        let tracker_clone = tracker.clone();
        let handle = tokio::spawn(async move {
            if i % 2 == 0 {
                tracker_clone.update_stats(StatsEvent::Peers, 1);
            } else {
                tracker_clone.update_stats(StatsEvent::Peers, -1);
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.await.unwrap();
    }
    let stats = tracker.get_stats();
    assert_eq!(stats.peers, 0, "Peers should be 0 after balanced operations");
}

#[tokio::test]
async fn test_stats_torrent_lifecycle() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();
    let peer = common::create_test_peer(
        peer_id,
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        6881,
    );
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    let stats_after_add = tracker.get_stats();
    assert_eq!(stats_after_add.torrents, 1, "Torrents should be 1");
    assert_eq!(stats_after_add.peers, 1, "Peers should be 1");
    tracker.remove_torrent_peer(info_hash, peer_id, false, false);
    let stats_after_remove = tracker.get_stats();
    assert_eq!(stats_after_remove.torrents, 0, "Torrents should be 0");
    assert_eq!(stats_after_remove.peers, 0, "Peers should be 0");
}

#[tokio::test]
async fn test_stats_overflow_protection() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    tracker.update_stats(StatsEvent::Peers, i64::MAX / 2);
    let stats = tracker.get_stats();
    assert!(stats.peers > 0, "Large increment should work");
    tracker.update_stats(StatsEvent::Peers, -(i64::MAX / 4));
    let stats_after = tracker.get_stats();
    assert!(stats_after.peers < stats.peers, "Large decrement should work");
}

#[tokio::test]
async fn test_stats_http_tcp_separation() {
    let tracker: common::TestTracker = common::create_test_tracker().await;
    tracker.update_stats(StatsEvent::Tcp4AnnouncesHandled, 10);
    tracker.update_stats(StatsEvent::Tcp4ScrapesHandled, 5);
    let stats = tracker.get_stats();
    assert_eq!(stats.tcp4_announces_handled, 10, "TCP4 announces should be tracked");
    assert_eq!(stats.tcp4_scrapes_handled, 5, "TCP4 scrapes should be tracked");
    tracker.update_stats(StatsEvent::Tcp6AnnouncesHandled, 3);
    let stats = tracker.get_stats();
    assert_eq!(stats.tcp6_announces_handled, 3, "TCP6 announces should be tracked");
}