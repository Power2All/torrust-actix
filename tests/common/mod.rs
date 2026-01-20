// Common test utilities and fixtures

use std::sync::Arc;
use torrust_actix::config::structs::api_trackers_config::ApiTrackersConfig;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::config::structs::database_config::DatabaseConfig;
use torrust_actix::config::structs::database_structure_config::DatabaseStructureConfig;
use torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig;
use torrust_actix::config::structs::sentry_config::SentryConfig;
use torrust_actix::config::structs::tracker_config::TrackerConfig;
use torrust_actix::config::structs::udp_trackers_config::UdpTrackersConfig;
use torrust_actix::database::enums::database_drivers::DatabaseDrivers;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
use tempfile::TempDir;

/// Create a test configuration with SQLite in-memory database
pub async fn create_test_config() -> Arc<Configuration> {
    let config = Configuration {
        log_level: "info".to_string(),
        log_console_interval: 60,
        tracker_config: TrackerConfig {
            announce_interval: 120,
            min_announce_interval: 60,
            max_peer_returned: 74,
            persistent: false,
        },
        sentry_config: SentryConfig {
            enabled: false,
            dsn: String::new(),
        },
        database: DatabaseConfig {
            engine: DatabaseDrivers::sqlite3,
            path: ":memory:".to_string(),
        },
        database_structure: DatabaseStructureConfig {
            create_database: false,
        },
        http_server: vec![],
        udp_server: vec![],
        api_server: vec![],
    };
    Arc::new(config)
}

/// Create a test HTTP trackers configuration
pub fn create_test_http_config() -> Arc<HttpTrackersConfig> {
    Arc::new(HttpTrackersConfig {
        bind_address: "127.0.0.1:8080".to_string(),
        ssl: false,
        ssl_key: String::new(),
        ssl_cert: String::new(),
    })
}

/// Create a test API trackers configuration
pub fn create_test_api_config() -> Arc<ApiTrackersConfig> {
    Arc::new(ApiTrackersConfig {
        bind_address: "127.0.0.1:8081".to_string(),
        ssl: false,
        ssl_key: String::new(),
        ssl_cert: String::new(),
        access_tokens: vec![],
    })
}

/// Create a test tracker instance
pub async fn create_test_tracker() -> Arc<TorrentTracker> {
    let config = create_test_config().await;
    Arc::new(TorrentTracker::new(config, false).await)
}

/// Create a temporary directory for test files
pub fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Generate a random InfoHash for testing
pub fn random_info_hash() -> torrust_actix::tracker::structs::info_hash::InfoHash {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 20] = rng.r#gen();
    torrust_actix::tracker::structs::info_hash::InfoHash(bytes)
}

/// Generate a random PeerId for testing
pub fn random_peer_id() -> torrust_actix::tracker::structs::peer_id::PeerId {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 20] = rng.r#gen();
    torrust_actix::tracker::structs::peer_id::PeerId(bytes)
}

/// Create a test torrent peer
pub fn create_test_peer(peer_id: torrust_actix::tracker::structs::peer_id::PeerId, ip: std::net::IpAddr, port: u16) -> torrust_actix::tracker::structs::torrent_peer::TorrentPeer {
    use torrust_actix::common::structs::number_of_bytes::NumberOfBytes;
    use torrust_actix::tracker::structs::torrent_peer::TorrentPeer;

    TorrentPeer {
        peer_id,
        peer_addr: std::net::SocketAddr::new(ip, port),
        updated: std::time::Instant::now(),
        uploaded: NumberOfBytes(0),
        downloaded: NumberOfBytes(0),
        left: NumberOfBytes(1000),
        event: torrust_actix::tracker::enums::announce_event::AnnounceEvent::Started,
    }
}
