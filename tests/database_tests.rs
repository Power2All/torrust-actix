mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use torrust_actix::common::common::hash_id;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::database::enums::database_drivers::DatabaseDrivers;
use torrust_actix::tracker::enums::updates_action::UpdatesAction;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
use torrust_actix::tracker::structs::user_entry_item::UserEntryItem;
use torrust_actix::tracker::structs::user_id::UserId;

async fn create_sqlite_test_config() -> Arc<Configuration> {
    let mut config: Configuration = Configuration::init();
    config.database.engine = DatabaseDrivers::sqlite3;
    config.database.persistent = true;
    config.database.path = ":memory:".to_string();
    Arc::new(config)
}

#[tokio::test]
async fn test_database_connector_creation() {
    let _config = create_sqlite_test_config().await;
}

#[tokio::test]
async fn test_load_torrents_from_empty_database() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let result: Result<(u64, u64), _> = tracker.sqlx.load_torrents(tracker.clone()).await;
    assert!(result.is_ok(), "Should load successfully even if empty");
    let (torrent_count, peer_count) = result.unwrap();
    assert_eq!(torrent_count, 0, "Should have 0 torrents initially");
    assert_eq!(peer_count, 0, "Should have 0 peers initially");
}

#[tokio::test]
async fn test_save_and_load_whitelist() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let info_hash1 = common::random_info_hash();
    let info_hash2 = common::random_info_hash();
    let whitelists = vec![
        (info_hash1, UpdatesAction::Add),
        (info_hash2, UpdatesAction::Add),
    ];
    let save_result: Result<u64, _> = tracker.sqlx.save_whitelist(tracker.clone(), whitelists).await;
    assert!(save_result.is_ok(), "Should save whitelist successfully");
    let load_result: Result<u64, _> = tracker.sqlx.load_whitelist(tracker.clone()).await;
    assert!(load_result.is_ok(), "Should load whitelist successfully");
    let count = load_result.unwrap();
    assert_eq!(count, 2, "Should load 2 whitelisted torrents");
}

#[tokio::test]
async fn test_load_lists_reads_a_read_only_whitelist() {
    // v4.2.24 loaded a list only when its table was persistent, so the usual read-only whitelist (the
    // torrents table, `persistent = false`) came up empty and every announce was refused.
    use torrust_actix::tracker::structs::torrent_update_data::TorrentUpdateData;
    // (whitelist_enabled, whitelist table persistent, expected to load)
    for (enabled, table_persistent, loads) in [(true, Some(false), true), (false, Some(true), true), (false, Some(false), false)] {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut config: Configuration = Configuration::init();
        config.database.engine = DatabaseDrivers::sqlite3;
        config.database.persistent = true;
        config.database.insert_vacant = true;
        config.database.path = temp_dir.path().join("lists.db").to_str().expect("Invalid temp path").to_string();
        config.tracker_config.whitelist_enabled = enabled;
        config.database_structure.whitelist.table_name = config.database_structure.torrents.table_name.clone();
        config.database_structure.whitelist.column_infohash = config.database_structure.torrents.column_infohash.clone();
        config.database_structure.whitelist.persistent = table_persistent;
        let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(Arc::new(config), true).await);
        let info_hash = common::random_info_hash();
        tracker.sqlx.save_torrents(tracker.clone(), &BTreeMap::from([(info_hash, (TorrentUpdateData::default(), UpdatesAction::Add))])).await.unwrap();
        TorrentTracker::load_lists(&tracker).await;
        assert_eq!(tracker.check_whitelist(info_hash), loads, "whitelist_enabled = {enabled}, persistent = {table_persistent:?}");
    }
}

#[tokio::test]
async fn test_save_and_load_blacklist() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let info_hash = common::random_info_hash();
    let blacklists = vec![(info_hash, UpdatesAction::Add)];
    let save_result: Result<u64, _> = tracker.sqlx.save_blacklist(tracker.clone(), blacklists).await;
    assert!(save_result.is_ok(), "Should save blacklist successfully");
    let load_result: Result<u64, _> = tracker.sqlx.load_blacklist(tracker.clone()).await;
    assert!(load_result.is_ok(), "Should load blacklist successfully");
    let count = load_result.unwrap();
    assert_eq!(count, 1, "Should load 1 blacklisted torrent");
}

#[tokio::test]
async fn test_save_and_load_keys() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let info_hash = common::random_info_hash();
    let mut keys = BTreeMap::new();
    keys.insert(info_hash, (chrono::Utc::now().timestamp() + 3600, UpdatesAction::Add));
    let save_result: Result<u64, _> = tracker.sqlx.save_keys(tracker.clone(), keys).await;
    assert!(save_result.is_ok(), "Should save keys successfully");
    let load_result: Result<u64, _> = tracker.sqlx.load_keys(tracker.clone()).await;
    assert!(load_result.is_ok(), "Should load keys successfully");
    let count = load_result.unwrap();
    assert_eq!(count, 1, "Should load 1 key");
}

#[tokio::test]
async fn test_database_optimization_no_clone() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let result: Result<(u64, u64), _> = tracker.sqlx.load_torrents(tracker.clone()).await;
    assert!(result.is_ok(), "Optimized database connector should work correctly");
}

#[tokio::test]
async fn test_database_update_action_add() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let info_hash = common::random_info_hash();
    let whitelists = vec![(info_hash, UpdatesAction::Add)];
    let _: u64 = tracker.sqlx.save_whitelist(tracker.clone(), whitelists).await.unwrap();
    let _: u64 = tracker.sqlx.load_whitelist(tracker.clone()).await.unwrap();
    let is_whitelisted = tracker.check_whitelist(info_hash);
    assert!(is_whitelisted, "InfoHash should be in whitelist after Add action");
}

#[tokio::test]
async fn test_database_update_action_remove() {
    let mut config: Configuration = Configuration::init();
    config.database.engine = DatabaseDrivers::sqlite3;
    config.database.persistent = true;
    config.database.path = ":memory:".to_string();
    // Required so save_whitelist actually issues a DELETE on UpdatesAction::Remove.
    config.database.remove_action = true;
    let config: Arc<Configuration> = Arc::new(config);
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let info_hash = common::random_info_hash();
    let _: u64 = tracker.sqlx.save_whitelist(tracker.clone(), vec![(info_hash, UpdatesAction::Add)]).await.unwrap();
    let _: u64 = tracker.sqlx.load_whitelist(tracker.clone()).await.unwrap();
    let _: u64 = tracker.sqlx.save_whitelist(tracker.clone(), vec![(info_hash, UpdatesAction::Remove)]).await.unwrap();
    // load_whitelist only adds rows it sees; clear the in-memory state first
    // so the post-remove load reflects only what's actually in the database.
    tracker.clear_whitelist();
    let _: u64 = tracker.sqlx.load_whitelist(tracker.clone()).await.unwrap();
    let is_whitelisted = tracker.check_whitelist(info_hash);
    assert!(!is_whitelisted, "InfoHash should not be in whitelist after Remove action");
}

#[tokio::test]
async fn test_reset_seeds_peers() {
    let config: Arc<Configuration> = create_sqlite_test_config().await;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let result: Result<(), _> = tracker.sqlx.reset_seeds_peers(tracker.clone()).await;
    assert!(result.is_ok(), "Reset should complete successfully");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_database_writes() {
    // Concurrent SQLite writes need a shared database file across pool
    // connections; `:memory:` gives each connection its own private DB.
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("concurrent_writes.db");
    let mut config: Configuration = Configuration::init();
    config.database.engine = DatabaseDrivers::sqlite3;
    config.database.persistent = true;
    config.database.path = db_path.to_str().expect("Invalid temp path").to_string();
    let config: Arc<Configuration> = Arc::new(config);
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(config, true).await);
    let mut handles = vec![];
    for _i in 0..10 {
        let tracker_clone: Arc<TorrentTracker> = tracker.clone();
        let handle = tokio::spawn(async move {
            let info_hash = common::random_info_hash();
            let whitelists = vec![(info_hash, UpdatesAction::Add)];
            let tracker_ref: Arc<TorrentTracker> = tracker_clone.clone();
            tracker_clone.sqlx.save_whitelist(tracker_ref, whitelists).await
        });
        handles.push(handle);
    }
    for handle in handles {
        let result: Result<u64, _> = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Concurrent writes should succeed");
    }
}

#[tokio::test]
async fn test_load_numeric_id_user_past_u32() {
    // A numeric-id user used to abort the SQLite boot load: the INTEGER id was decoded as bytes,
    // and uploaded/downloaded as u32, so the first user past 4 GiB took the tracker down for good.
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut config: Configuration = Configuration::init();
    config.database.engine = DatabaseDrivers::sqlite3;
    config.database.persistent = true;
    config.database.path = temp_dir.path().join("users.db").to_str().expect("Invalid temp path").to_string();
    config.database.insert_vacant = true;
    config.database_structure.users.id_uuid = false;
    let tracker: Arc<TorrentTracker> = Arc::new(TorrentTracker::new(Arc::new(config), true).await);
    let user_hash = UserId(hash_id("7"));
    let user = UserEntryItem {
        key: UserId([1; 20]),
        user_id: Some(7),
        user_uuid: None,
        uploaded: 5 << 30,
        downloaded: 1,
        completed: 2,
        updated: 1_700_000_000,
        active: 1,
        torrents_active: BTreeMap::new(),
    };
    tracker.sqlx.save_users(tracker.clone(), BTreeMap::from([(user_hash, (user, UpdatesAction::Add))])).await.unwrap();
    tracker.clear_users();
    assert_eq!(tracker.sqlx.load_users(tracker.clone()).await.unwrap(), 1);
    let loaded = tracker.get_user(user_hash).expect("user should load under the API's hash_id(\"7\") key");
    assert_eq!(loaded.user_id, Some(7));
    assert_eq!(loaded.uploaded, 5 << 30);
}
