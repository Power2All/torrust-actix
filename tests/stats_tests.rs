// Integration tests for Statistics and Metrics

mod common;

use torrust_actix::stats::enums::stats_event::StatsEvent;

#[tokio::test]
async fn test_stats_initial_values() {
    let tracker = common::create_test_tracker().await;

    // Initial stats should be zero or consistent
    let stats = tracker.get_stats();

    assert_eq!(stats.torrents, 0, "Initial torrents count should be 0");
    assert_eq!(stats.seeds, 0, "Initial seeds count should be 0");
    assert_eq!(stats.peers, 0, "Initial peers count should be 0");
    assert_eq!(stats.completed, 0, "Initial completed count should be 0");
}

#[tokio::test]
async fn test_stats_increment_decrement() {
    let tracker = common::create_test_tracker().await;

    // Test increment
    tracker.update_stats(StatsEvent::Torrents, 1);
    tracker.update_stats(StatsEvent::Seeds, 5);
    tracker.update_stats(StatsEvent::Peers, 10);

    let stats = tracker.get_stats();
    assert_eq!(stats.torrents, 1, "Torrents should be 1");
    assert_eq!(stats.seeds, 5, "Seeds should be 5");
    assert_eq!(stats.peers, 10, "Peers should be 10");

    // Test decrement
    tracker.update_stats(StatsEvent::Seeds, -2);
    tracker.update_stats(StatsEvent::Peers, -3);

    let stats = tracker.get_stats();
    assert_eq!(stats.seeds, 3, "Seeds should be 3 after decrement");
    assert_eq!(stats.peers, 7, "Peers should be 7 after decrement");
}

#[tokio::test]
async fn test_stats_concurrent_updates() {
    let tracker = common::create_test_tracker().await;

    // Perform concurrent stats updates
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
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();

    // Add a peer with completed event
    let peer = common::create_test_peer(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        6881,
    );

    tracker.add_torrent_peer(info_hash, peer_id, peer, true);

    let stats = tracker.get_stats();
    assert_eq!(stats.completed, 1, "Completed count should increment");
}

#[tokio::test]
async fn test_stats_seed_peer_ratio() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();

    // Add seeds
    for i in 0..5 {
        let peer_id = common::random_peer_id();
        let mut peer = common::create_test_peer(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            6881 + i,
        );
        peer.left = torrust_actix::common::structs::number_of_bytes::NumberOfBytes(0);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }

    // Add peers
    for i in 0..10 {
        let peer_id = common::random_peer_id();
        let mut peer = common::create_test_peer(
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
    let tracker = common::create_test_tracker().await;

    // Add some data
    tracker.update_stats(StatsEvent::Torrents, 5);
    tracker.update_stats(StatsEvent::Seeds, 10);
    tracker.update_stats(StatsEvent::Peers, 20);

    let prometheus_output = tracker.get_stats_prometheus();

    // Verify Prometheus format
    assert!(prometheus_output.contains("torrents"), "Should contain torrents metric");
    assert!(prometheus_output.contains("seeds"), "Should contain seeds metric");
    assert!(prometheus_output.contains("peers"), "Should contain peers metric");
}

#[tokio::test]
async fn test_stats_atomic_operations() {
    let tracker = common::create_test_tracker().await;

    // Test that stats updates are atomic by performing rapid increments and decrements
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
    // With 25 increments and 25 decrements, should be 0
    assert_eq!(stats.peers, 0, "Peers should be 0 after balanced operations");
}

#[tokio::test]
async fn test_stats_torrent_lifecycle() {
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();
    let peer_id = common::random_peer_id();

    // Add peer (torrent created)
    let peer = common::create_test_peer(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        6881,
    );
    tracker.add_torrent_peer(info_hash, peer_id, peer, false);

    let stats_after_add = tracker.get_stats();
    assert_eq!(stats_after_add.torrents, 1, "Torrents should be 1");
    assert_eq!(stats_after_add.peers, 1, "Peers should be 1");

    // Remove peer (torrent removed if non-persistent)
    tracker.remove_torrent_peer(info_hash, peer_id, false, false);

    let stats_after_remove = tracker.get_stats();
    assert_eq!(stats_after_remove.torrents, 0, "Torrents should be 0");
    assert_eq!(stats_after_remove.peers, 0, "Peers should be 0");
}

#[tokio::test]
async fn test_stats_overflow_protection() {
    let tracker = common::create_test_tracker().await;

    // Test large increments don't cause issues
    tracker.update_stats(StatsEvent::Peers, i64::MAX / 2);
    let stats = tracker.get_stats();

    assert!(stats.peers > 0, "Large increment should work");

    // Test that decrements work with large values
    tracker.update_stats(StatsEvent::Peers, -(i64::MAX / 4));
    let stats_after = tracker.get_stats();

    assert!(stats_after.peers < stats.peers, "Large decrement should work");
}

#[tokio::test]
async fn test_stats_http_tcp_separation() {
    let tracker = common::create_test_tracker().await;

    // Test HTTP stats
    tracker.update_stats(StatsEvent::Tcp4Announces, 10);
    tracker.update_stats(StatsEvent::Tcp4Scrapes, 5);

    let stats = tracker.get_stats();
    assert_eq!(stats.tcp4announces, 10, "TCP4 announces should be tracked");
    assert_eq!(stats.tcp4scrapes, 5, "TCP4 scrapes should be tracked");

    // Test IPv6 stats
    tracker.update_stats(StatsEvent::Tcp6Announces, 3);
    let stats = tracker.get_stats();
    assert_eq!(stats.tcp6announces, 3, "TCP6 announces should be tracked");
}
