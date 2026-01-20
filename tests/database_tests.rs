mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::database::enums::database_drivers::DatabaseDrivers;
use torrust_actix::database::structs::database_connector::DatabaseConnector;
use torrust_actix::tracker::enums::updates_action::UpdatesAction;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

async fn create_sqlite_test_config() -> Arc<Configuration> {
    let mut config = Configuration::default();
    config.database.engine = DatabaseDrivers::sqlite3;
    config.database.persistent = true;
    config.database.path = ":memory:".to_string(); // In-memory SQLite for testing
    Arc::new(config)
}

#[tokio::test]
async fn test_database_connector_creation() {
    let config = create_sqlite_test_config().await;
    let connector = DatabaseConnector::new(config, true).await;

    // Verify connector was created without errors
    assert!(connector.engine.is_some(), "Database engine should be set");
}

#[tokio::test]
async fn test_load_torrents_from_empty_database() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    let result = tracker.sqlx.load_torrents(tracker.clone()).await;

    assert!(result.is_ok(), "Should load successfully even if empty");
    let (torrent_count, peer_count) = result.unwrap();
    assert_eq!(torrent_count, 0, "Should have 0 torrents initially");
    assert_eq!(peer_count, 0, "Should have 0 peers initially");
}

#[tokio::test]
async fn test_save_and_load_whitelist() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    // Create test whitelist
    let info_hash1 = common::random_info_hash();
    let info_hash2 = common::random_info_hash();

    let whitelists = vec![
        (info_hash1, UpdatesAction::Add),
        (info_hash2, UpdatesAction::Add),
    ];

    // Save whitelist
    let save_result = tracker.sqlx.save_whitelist(tracker.clone(), whitelists).await;
    assert!(save_result.is_ok(), "Should save whitelist successfully");

    // Load whitelist
    let load_result = tracker.sqlx.load_whitelist(tracker.clone()).await;
    assert!(load_result.is_ok(), "Should load whitelist successfully");

    let count = load_result.unwrap();
    assert_eq!(count, 2, "Should load 2 whitelisted torrents");
}

#[tokio::test]
async fn test_save_and_load_blacklist() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    let info_hash = common::random_info_hash();
    let blacklists = vec![(info_hash, UpdatesAction::Add)];

    let save_result = tracker.sqlx.save_blacklist(tracker.clone(), blacklists).await;
    assert!(save_result.is_ok(), "Should save blacklist successfully");

    let load_result = tracker.sqlx.load_blacklist(tracker.clone()).await;
    assert!(load_result.is_ok(), "Should load blacklist successfully");

    let count = load_result.unwrap();
    assert_eq!(count, 1, "Should load 1 blacklisted torrent");
}

#[tokio::test]
async fn test_save_and_load_keys() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    let info_hash = common::random_info_hash();
    let mut keys = BTreeMap::new();
    keys.insert(info_hash, (chrono::Utc::now().timestamp() + 3600, UpdatesAction::Add));

    let save_result = tracker.sqlx.save_keys(tracker.clone(), keys).await;
    assert!(save_result.is_ok(), "Should save keys successfully");

    let load_result = tracker.sqlx.load_keys(tracker.clone()).await;
    assert!(load_result.is_ok(), "Should load keys successfully");

    let count = load_result.unwrap();
    assert_eq!(count, 1, "Should load 1 key");
}

#[tokio::test]
async fn test_database_optimization_no_clone() {
    // This test verifies that the optimization (using as_ref() instead of clone().unwrap()) works
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    // This should work without panicking (tests the new pattern matching approach)
    let result = tracker.sqlx.load_torrents(tracker.clone()).await;
    assert!(result.is_ok(), "Optimized database connector should work correctly");
}

#[tokio::test]
async fn test_database_update_action_add() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    let info_hash = common::random_info_hash();

    // Add to whitelist
    let whitelists = vec![(info_hash, UpdatesAction::Add)];
    tracker.sqlx.save_whitelist(tracker.clone(), whitelists).await.unwrap();

    // Verify it was added
    tracker.sqlx.load_whitelist(tracker.clone()).await.unwrap();
    let is_whitelisted = tracker.is_info_hash_whitelisted(info_hash);
    assert!(is_whitelisted, "InfoHash should be in whitelist after Add action");
}

#[tokio::test]
async fn test_database_update_action_remove() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    let info_hash = common::random_info_hash();

    // First add
    tracker.sqlx.save_whitelist(tracker.clone(), vec![(info_hash, UpdatesAction::Add)]).await.unwrap();
    tracker.sqlx.load_whitelist(tracker.clone()).await.unwrap();

    // Then remove
    tracker.sqlx.save_whitelist(tracker.clone(), vec![(info_hash, UpdatesAction::Remove)]).await.unwrap();
    tracker.sqlx.load_whitelist(tracker.clone()).await.unwrap();

    let is_whitelisted = tracker.is_info_hash_whitelisted(info_hash);
    assert!(!is_whitelisted, "InfoHash should not be in whitelist after Remove action");
}

#[tokio::test]
async fn test_reset_seeds_peers() {
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    let result = tracker.sqlx.reset_seeds_peers(tracker.clone()).await;
    assert!(result.is_ok(), "Reset should complete successfully");
}

#[tokio::test]
async fn test_concurrent_database_writes() {
    // Test the parallel database update optimization
    let config = create_sqlite_test_config().await;
    let tracker = Arc::new(TorrentTracker::new(config, true).await);

    // Spawn multiple concurrent database writes
    let mut handles = vec![];

    for i in 0..10 {
        let tracker_clone = tracker.clone();
        let handle = tokio::spawn(async move {
            let info_hash = common::random_info_hash();
            let whitelists = vec![(info_hash, UpdatesAction::Add)];
            tracker_clone.sqlx.save_whitelist(tracker_clone, whitelists).await
        });
        handles.push(handle);
    }

    // Wait for all writes to complete
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Concurrent writes should succeed");
    }
}
