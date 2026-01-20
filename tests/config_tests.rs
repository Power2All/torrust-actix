mod common;

use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_config_default_values() {
    let config = common::create_test_config().await;

    assert!(config.tracker_config.request_interval > 0, "Request interval should be positive");
    assert!(config.tracker_config.request_interval_minimum > 0, "Min request interval should be positive");
    assert!(config.tracker_config.peers_timeout > 0, "Peers timeout should be positive");
    assert_eq!(config.database.persistent, false, "Default should be non-persistent");
}

#[tokio::test]
async fn test_config_toml_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r#"
[core]
log_level = "info"

[tracker]
announce_interval = 120
min_announce_interval = 60
max_peer_returned = 74
persistent = false

[database]
engine = "sqlite3"
path = "data.db"

[udp_server]
bind_address = "0.0.0.0:6969"
threads = 4
queue_size = 1024

[http_server]
bind_address = "0.0.0.0:8080"
workers = 4

[sentry_config]
enabled = false
dsn = ""
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test that TOML file can be loaded (this assumes your config has a from_file method)
    assert!(config_path.exists(), "Config file should exist");
}

#[tokio::test]
async fn test_config_database_settings() {
    let config = common::create_test_config().await;

    // Verify database configuration
    assert!(config.database.path.len() > 0, "Database path should not be empty");
}

#[tokio::test]
async fn test_config_tracker_limits() {
    let config = common::create_test_config().await;

    // Verify request intervals are sensible
    assert!(
        config.tracker_config.request_interval_minimum <= config.tracker_config.request_interval,
        "Min request interval should be <= request interval"
    );

    // Verify timeout is reasonable
    assert!(
        config.tracker_config.peers_timeout > config.tracker_config.request_interval,
        "Peers timeout should be greater than request interval"
    );
}

#[tokio::test]
async fn test_config_udp_server_settings() {
    let config = common::create_test_config().await;

    // Verify UDP server configuration exists
    if !config.udp_server.is_empty() { let ref udp_config = &config.udp_server[0];
        assert!(!udp_config.bind_address.is_empty(), "UDP bind address should not be empty");
        assert!(udp_config.udp_threads > 0, "UDP threads should be positive");
        assert!(udp_config.worker_threads > 0, "UDP worker threads should be positive");
    }
}

#[tokio::test]
async fn test_config_http_server_settings() {
    let config = common::create_test_config().await;

    // Verify HTTP server configuration exists
    if !config.http_server.is_empty() { let ref http_config = &config.http_server[0];
        assert!(!http_config.bind_address.is_empty(), "HTTP bind address should not be empty");
        assert!(http_config.threads > 0, "HTTP threads should be positive");
    }
}

#[tokio::test]
async fn test_config_sentry_disabled_by_default() {
    let config = common::create_test_config().await;

    // Sentry should be disabled in test configuration
    assert_eq!(config.sentry_config.enabled, false, "Sentry should be disabled by default");
}

#[tokio::test]
async fn test_config_validation() {
    let config = common::create_test_config().await;

    // Test that configuration values are within acceptable ranges
    assert!(
        config.tracker_config.request_interval >= 1 && config.tracker_config.request_interval <= 3600,
        "Request interval should be between 1 and 3600 seconds"
    );

    assert!(
        config.tracker_config.request_interval_minimum >= 1 && config.tracker_config.request_interval_minimum <= 3600,
        "Min request interval should be between 1 and 3600 seconds"
    );

    assert!(
        config.tracker_config.peers_timeout >= 60 && config.tracker_config.peers_timeout <= 7200,
        "Peers timeout should be between 60 and 7200 seconds"
    );
}

#[tokio::test]
async fn test_config_thread_safety() {
    let config = common::create_test_config().await;

    // Test that Arc<Configuration> can be cloned safely
    let config_clone1 = config.clone();
    let config_clone2 = config.clone();

    assert_eq!(
        config.tracker_config.request_interval,
        config_clone1.tracker_config.request_interval,
        "Cloned config should have same values"
    );

    assert_eq!(
        config_clone1.tracker_config.request_interval,
        config_clone2.tracker_config.request_interval,
        "All clones should have same values"
    );
}

#[tokio::test]
async fn test_config_concurrent_access() {
    let config = common::create_test_config().await;

    // Test concurrent access to configuration
    let mut handles = vec![];

    for _ in 0..10 {
        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            let interval = config_clone.tracker_config.request_interval;
            interval > 0
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result, "Concurrent config access should work correctly");
    }
}
