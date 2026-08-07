mod common;

use std::fs;
use tempfile::TempDir;
use torrust_actix::config::structs::configuration::Configuration;

#[tokio::test]
async fn test_config_default_values() {
    let config: common::TestConfig = common::create_test_config().await;
    assert!(config.tracker_config.request_interval > 0, "Request interval should be positive");
    assert!(config.tracker_config.request_interval_minimum > 0, "Min request interval should be positive");
    assert!(config.tracker_config.peers_timeout > 0, "Peers timeout should be positive");
    assert!(!config.database.persistent, "Default should be non-persistent");
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
    assert!(config_path.exists(), "Config file should exist");
}

#[tokio::test]
async fn test_config_database_settings() {
    let config: common::TestConfig = common::create_test_config().await;
    assert!(!config.database.path.is_empty(), "Database path should not be empty");
}

#[tokio::test]
async fn test_config_tracker_limits() {
    let config: common::TestConfig = common::create_test_config().await;
    assert!(
        config.tracker_config.request_interval_minimum <= config.tracker_config.request_interval,
        "Min request interval should be <= request interval"
    );
    assert!(
        config.tracker_config.peers_timeout > config.tracker_config.request_interval,
        "Peers timeout should be greater than request interval"
    );
}

#[tokio::test]
async fn test_config_udp_server_settings() {
    let config: common::TestConfig = common::create_test_config().await;
    if !config.udp_server.is_empty() { let udp_config = &&config.udp_server[0];
        assert!(!udp_config.bind_address.is_empty(), "UDP bind address should not be empty");
        assert!(udp_config.udp_threads > 0, "UDP threads should be positive");
        assert!(udp_config.worker_threads > 0, "UDP worker threads should be positive");
    }
}

#[tokio::test]
async fn test_config_http_server_settings() {
    let config: common::TestConfig = common::create_test_config().await;
    if !config.http_server.is_empty() { let http_config = &&config.http_server[0];
        assert!(!http_config.bind_address.is_empty(), "HTTP bind address should not be empty");
        assert!(http_config.threads > 0, "HTTP threads should be positive");
    }
}

#[tokio::test]
async fn test_config_sentry_disabled_by_default() {
    let config: common::TestConfig = common::create_test_config().await;
    assert!(!config.sentry_config.enabled, "Sentry should be disabled by default");
}

#[test]
fn test_validate_sentry_accepts_rates_in_range() {
    let config = common::build_test_config(|c| {
        c.sentry_config.enabled = true;
        c.sentry_config.dsn = "https://key@example.invalid/1".to_string();
        c.sentry_config.sample_rate = 0.0;
        c.sentry_config.traces_sample_rate = 1.0;
    });
    Configuration::validate_sentry(&config);
}

#[test]
#[should_panic(expected = "sentry.sample_rate must be between 0.0 and 1.0")]
fn test_validate_sentry_rejects_out_of_range_sample_rate() {
    // sentry 0.49's ClientOptions::sample_rate builder panics outside [0.0, 1.0],
    // so this must be caught at config-validation time instead.
    let config = common::build_test_config(|c| {
        c.sentry_config.enabled = true;
        c.sentry_config.dsn = "https://key@example.invalid/1".to_string();
        c.sentry_config.sample_rate = 1.5;
    });
    Configuration::validate_sentry(&config);
}

#[test]
#[should_panic(expected = "sentry.traces_sample_rate must be between 0.0 and 1.0")]
fn test_validate_sentry_rejects_out_of_range_traces_sample_rate() {
    let config = common::build_test_config(|c| {
        c.sentry_config.enabled = true;
        c.sentry_config.dsn = "https://key@example.invalid/1".to_string();
        c.sentry_config.traces_sample_rate = -0.1;
    });
    Configuration::validate_sentry(&config);
}

#[tokio::test]
async fn test_config_validation() {
    let config: common::TestConfig = common::create_test_config().await;
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
    let config: common::TestConfig = common::create_test_config().await;
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
    let config: common::TestConfig = common::create_test_config().await;
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

#[tokio::test]
async fn test_http_config_rtctorrent_defaults_to_false() {
    let config = common::create_test_config().await;
    for (i, http) in config.http_server.iter().enumerate() {
        assert!(
            !http.rtctorrent,
            "http_server[{}].rtctorrent should default to false", i
        );
    }
}

#[tokio::test]
async fn test_http_config_rtctorrent_can_be_enabled() {
    let http_config = common::create_test_http_config_with_rtctorrent(true);
    assert!(http_config.rtctorrent, "rtctorrent should be true when explicitly set");
}

#[tokio::test]
async fn test_http_config_rtctorrent_disabled_by_default_helper() {
    let http_config = common::create_test_http_config();
    assert!(!http_config.rtctorrent, "Default test http config should have rtctorrent=false");
}

#[tokio::test]
async fn test_config_rtctorrent_toml_default_missing_field() {
    use torrust_actix::config::structs::configuration::Configuration;
    let toml = r#"
log_level = "info"
log_console_interval = 60
udp_server = []
api_server = []

[tracker_config]
api_key = "TestKeyThatIsLongEnoughToBeValid1234!"
whitelist_enabled = false
blacklist_enabled = false
keys_enabled = false
keys_cleanup_interval = 60
users_enabled = false
request_interval = 1800
request_interval_minimum = 1800
peers_timeout = 2700
peers_cleanup_interval = 900
peers_cleanup_threads = 256
total_downloads = 0
swagger = false
prometheus_id = "test"
cluster = "standalone"
cluster_encoding = "binary"
cluster_token = ""
cluster_bind_address = "0.0.0.0:8888"
cluster_master_address = ""
cluster_keep_alive = 60
cluster_request_timeout = 15
cluster_disconnect_timeout = 15
cluster_reconnect_interval = 5
cluster_max_connections = 100
cluster_threads = 4
cluster_ssl = false
cluster_ssl_key = ""
cluster_ssl_cert = ""
cluster_tls_connection_rate = 64
rtc_interval = 10
rtc_peers_timeout = 120

[sentry_config]
enabled = false
dsn = ""
debug = false
sample_rate = 1.0
max_breadcrumbs = 100
attach_stacktrace = true
send_default_pii = false
traces_sample_rate = 1.0

[database]
engine = "sqlite3"
path = "sqlite://data.db"
persistent = false
persistent_interval = 60
insert_vacant = false
remove_action = false
update_completed = true
update_peers = false

[database_structure.torrents]
table_name = "torrents"
column_infohash = "infohash"
bin_type_infohash = true
column_seeds = "seeds"
column_peers = "peers"
column_completed = "completed"

[database_structure.whitelist]
table_name = "whitelist"
column_infohash = "infohash"
bin_type_infohash = true

[database_structure.blacklist]
table_name = "blacklist"
column_infohash = "infohash"
bin_type_infohash = true

[database_structure.keys]
table_name = "keys"
column_hash = "hash"
bin_type_hash = true
column_timeout = "timeout"

[database_structure.users]
table_name = "users"
id_uuid = true
column_uuid = "uuid"
column_id = "id"
column_key = "key"
bin_type_key = true
column_uploaded = "uploaded"
column_downloaded = "downloaded"
column_completed = "completed"
column_updated = "updated"
column_active = "active"

[[http_server]]
enabled = true
bind_address = "0.0.0.0:6969"
real_ip = "X-Real-IP"
trusted_proxies = false
keep_alive = 60
request_timeout = 15
disconnect_timeout = 15
max_connections = 1000
threads = 4
ssl = false
ssl_key = ""
ssl_cert = ""
tls_connection_rate = 64
# Note: no 'rtctorrent' field — should default to false
"#;
    let cfg = Configuration::load(toml.as_bytes())
        .expect("Config with missing rtctorrent should parse successfully");
    assert!(
        !cfg.http_server[0].rtctorrent,
        "Missing 'rtctorrent' key should deserialise as false"
    );
}

#[tokio::test]
async fn test_config_rtctorrent_toml_explicit_true() {
    use torrust_actix::config::structs::configuration::Configuration;
    let toml = r#"
log_level = "info"
log_console_interval = 60
udp_server = []
api_server = []

[tracker_config]
api_key = "TestKeyThatIsLongEnoughToBeValid1234!"
whitelist_enabled = false
blacklist_enabled = false
keys_enabled = false
keys_cleanup_interval = 60
users_enabled = false
request_interval = 1800
request_interval_minimum = 1800
peers_timeout = 2700
peers_cleanup_interval = 900
peers_cleanup_threads = 256
total_downloads = 0
swagger = false
prometheus_id = "test"
cluster = "standalone"
cluster_encoding = "binary"
cluster_token = ""
cluster_bind_address = "0.0.0.0:8888"
cluster_master_address = ""
cluster_keep_alive = 60
cluster_request_timeout = 15
cluster_disconnect_timeout = 15
cluster_reconnect_interval = 5
cluster_max_connections = 100
cluster_threads = 4
cluster_ssl = false
cluster_ssl_key = ""
cluster_ssl_cert = ""
cluster_tls_connection_rate = 64
rtc_interval = 10
rtc_peers_timeout = 120

[sentry_config]
enabled = false
dsn = ""
debug = false
sample_rate = 1.0
max_breadcrumbs = 100
attach_stacktrace = true
send_default_pii = false
traces_sample_rate = 1.0

[database]
engine = "sqlite3"
path = "sqlite://data.db"
persistent = false
persistent_interval = 60
insert_vacant = false
remove_action = false
update_completed = true
update_peers = false

[database_structure.torrents]
table_name = "torrents"
column_infohash = "infohash"
bin_type_infohash = true
column_seeds = "seeds"
column_peers = "peers"
column_completed = "completed"

[database_structure.whitelist]
table_name = "whitelist"
column_infohash = "infohash"
bin_type_infohash = true

[database_structure.blacklist]
table_name = "blacklist"
column_infohash = "infohash"
bin_type_infohash = true

[database_structure.keys]
table_name = "keys"
column_hash = "hash"
bin_type_hash = true
column_timeout = "timeout"

[database_structure.users]
table_name = "users"
id_uuid = true
column_uuid = "uuid"
column_id = "id"
column_key = "key"
bin_type_key = true
column_uploaded = "uploaded"
column_downloaded = "downloaded"
column_completed = "completed"
column_updated = "updated"
column_active = "active"

[[http_server]]
enabled = true
bind_address = "0.0.0.0:6969"
real_ip = "X-Real-IP"
trusted_proxies = false
keep_alive = 60
request_timeout = 15
disconnect_timeout = 15
max_connections = 1000
threads = 4
ssl = false
ssl_key = ""
ssl_cert = ""
tls_connection_rate = 64
rtctorrent = true
"#;
    let cfg = Configuration::load(toml.as_bytes())
        .expect("Config with rtctorrent=true should parse successfully");
    assert!(
        cfg.http_server[0].rtctorrent,
        "'rtctorrent = true' in TOML should deserialise as true"
    );
}
#[tokio::test]
async fn test_generated_config_round_trips() {
    // Every field the defaults emit has to parse back, and `#[serde(skip)]` fields must not
    // appear at all.
    let defaults = Configuration::init();
    let annotated = Configuration::generate_annotated_config(&defaults);
    assert!(annotated.contains("max_peers_per_torrent"), "new tracker limits must be written out");
    assert!(annotated.contains("trusted_proxy_ips"), "proxy allowlist must be written out");
    assert!(annotated.contains("proxy_addresses"), "UDP proxy allowlist must be written out");
    assert!(!annotated.contains("trusted_proxy_addrs"), "the parsed-only field must not be serialised");
    assert!(!annotated.contains("proxy_addrs ="), "the parsed-only field must not be serialised");

    let reparsed: Configuration = toml::from_str(&annotated).expect("generated config must parse");
    assert_eq!(reparsed.tracker_config.max_peers_per_torrent, defaults.tracker_config.max_peers_per_torrent);
    assert_eq!(reparsed.tracker_config.max_rtc_pending_answers, defaults.tracker_config.max_rtc_pending_answers);
    assert_eq!(reparsed.http_server[0].trusted_proxy_ips, defaults.http_server[0].trusted_proxy_ips);
    assert_eq!(reparsed.udp_server[0].proxy_addresses, defaults.udp_server[0].proxy_addresses);
}

#[tokio::test]
async fn test_new_limits_default_when_absent_from_toml() {
    // Config files predating these keys must fall back to safe defaults, not a disabled limit.
    let toml_str = r#"
log_level = "info"
log_console_interval = 60

[tracker_config]
api_key = "SomeVeryStrongApiKey123456789abc"
request_interval = 1800
request_interval_minimum = 1800
peers_timeout = 2700
peers_cleanup_interval = 900
peers_cleanup_threads = 256
total_downloads = 0

[database]
engine = "sqlite3"
path = "sqlite://data.db"
persistent = false
persistent_interval = 60
insert_vacant = false
remove_action = false
update_completed = true
update_peers = false

[database_structure]

[[http_server]]
enabled = true
bind_address = "0.0.0.0:6969"
real_ip = "X-Real-IP"
keep_alive = 60
request_timeout = 15
disconnect_timeout = 15
max_connections = 25000
threads = 4
ssl = false
ssl_key = ""
ssl_cert = ""
tls_connection_rate = 256

[[udp_server]]
enabled = true
bind_address = "0.0.0.0:6969"
udp_threads = 2
worker_threads = 4
receive_buffer_size = 134217728
send_buffer_size = 67108864
reuse_address = true

[[api_server]]
enabled = true
bind_address = "0.0.0.0:8080"
real_ip = "X-Real-IP"
keep_alive = 60
request_timeout = 30
disconnect_timeout = 30
max_connections = 25000
threads = 4
ssl = false
ssl_key = ""
ssl_cert = ""
tls_connection_rate = 256
"#;
    let config: Configuration = toml::from_str(toml_str).expect("legacy config must still parse");
    assert_eq!(config.tracker_config.max_peers_per_torrent, 10_000, "peer cap must default on, not off");
    assert_eq!(config.tracker_config.max_rtc_pending_answers, 32);
    assert!(config.http_server[0].trusted_proxy_ips.is_empty());
    assert!(config.udp_server[0].proxy_addresses.is_empty());
}
