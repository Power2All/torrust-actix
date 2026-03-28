#![allow(dead_code)]
use rand::RngExt;
use std::sync::Arc;
use tempfile::TempDir;
use torrust_actix::config::structs::api_trackers_config::ApiTrackersConfig;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::config::structs::http_trackers_config::HttpTrackersConfig;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

pub type TestTracker = Arc<TorrentTracker>;
pub type TestConfig = Arc<Configuration>;

pub async fn create_test_config() -> TestConfig {
    let mut config: Configuration = Configuration::init();
    config.database.path = ":memory:".to_string();
    config.database.persistent = false;
    Arc::new(config)
}

pub fn create_test_http_config() -> Arc<HttpTrackersConfig> {
    create_test_http_config_with_rtctorrent(false)
}

pub fn create_test_http_config_with_rtctorrent(rtctorrent: bool) -> Arc<HttpTrackersConfig> {
    Arc::new(HttpTrackersConfig {
        enabled: true,
        bind_address: "127.0.0.1:8080".to_string(),
        real_ip: String::new(),
        trusted_proxies: false,
        keep_alive: 5,
        request_timeout: 10,
        disconnect_timeout: 5,
        max_connections: 1000,
        threads: 4,
        ssl: false,
        ssl_key: String::new(),
        ssl_cert: String::new(),
        tls_connection_rate: 100,
        rtctorrent,
    })
}

pub fn create_test_api_config() -> Arc<ApiTrackersConfig> {
    Arc::new(ApiTrackersConfig {
        enabled: true,
        bind_address: "127.0.0.1:8081".to_string(),
        real_ip: String::new(),
        trusted_proxies: false,
        keep_alive: 5,
        request_timeout: 10,
        disconnect_timeout: 5,
        max_connections: 1000,
        threads: 4,
        ssl: false,
        ssl_key: String::new(),
        ssl_cert: String::new(),
        tls_connection_rate: 100,
    })
}

pub async fn create_test_tracker() -> TestTracker {
    let config: TestConfig = create_test_config().await;
    Arc::new(TorrentTracker::new(config, false).await)
}

pub fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

pub fn random_info_hash() -> torrust_actix::tracker::structs::info_hash::InfoHash {
    let mut rng = rand::rng();
    let bytes: [u8; 20] = rng.random();
    torrust_actix::tracker::structs::info_hash::InfoHash(bytes)
}

pub fn random_peer_id() -> torrust_actix::tracker::structs::peer_id::PeerId {
    let mut rng = rand::rng();
    let bytes: [u8; 20] = rng.random();
    torrust_actix::tracker::structs::peer_id::PeerId(bytes)
}

pub fn create_test_peer(
    peer_id: torrust_actix::tracker::structs::peer_id::PeerId,
    ip: std::net::IpAddr,
    port: u16
) -> torrust_actix::tracker::structs::torrent_peer::TorrentPeer {
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
        rtc_data: None,
    }
}

pub fn create_rtc_peer(
    peer_id: torrust_actix::tracker::structs::peer_id::PeerId,
    ip: std::net::IpAddr,
    port: u16,
    sdp_offer: Option<String>,
    left: i64,
) -> torrust_actix::tracker::structs::torrent_peer::TorrentPeer {
    use torrust_actix::common::structs::compressed_bytes::CompressedBytes;
    use torrust_actix::common::structs::number_of_bytes::NumberOfBytes;
    use torrust_actix::tracker::structs::rtc_data::RtcData;
    use torrust_actix::tracker::structs::torrent_peer::TorrentPeer;
    TorrentPeer {
        peer_id,
        peer_addr: std::net::SocketAddr::new(ip, port),
        updated: std::time::Instant::now(),
        uploaded: NumberOfBytes(0),
        downloaded: NumberOfBytes(0),
        left: NumberOfBytes(left),
        event: torrust_actix::tracker::enums::announce_event::AnnounceEvent::Started,
        rtc_data: Some(Box::new(RtcData {
            sdp_offer: sdp_offer.as_deref().map(CompressedBytes::compress),
            sdp_answer: None,
            connection_status: "pending".to_string(),
            pending_answers: Vec::new(),
        })),
    }
}

/// Subtract `duration` from `instant` without panicking on underflow.
///
/// `Instant::now() - large_duration` panics on Windows when the system uptime
/// is shorter than `large_duration`.  This helper halves the duration until
/// `checked_sub` succeeds, returning the oldest reachable instant.
fn saturating_instant_sub(instant: std::time::Instant, mut duration: std::time::Duration) -> std::time::Instant {
    loop {
        if let Some(t) = instant.checked_sub(duration) {
            return t;
        }
        if duration.is_zero() {
            return instant;
        }
        duration /= 2;
    }
}

/// Create a peer whose `updated` timestamp is `age` in the past, so cleanup will treat it as timed-out.
pub fn create_aged_peer(
    peer_id: torrust_actix::tracker::structs::peer_id::PeerId,
    ip: std::net::IpAddr,
    port: u16,
    age: std::time::Duration,
) -> torrust_actix::tracker::structs::torrent_peer::TorrentPeer {
    let mut peer = create_test_peer(peer_id, ip, port);
    peer.updated = saturating_instant_sub(std::time::Instant::now(), age);
    peer
}

/// Create a seed (left=0) whose `updated` timestamp is `age` in the past.
pub fn create_aged_seed(
    peer_id: torrust_actix::tracker::structs::peer_id::PeerId,
    ip: std::net::IpAddr,
    port: u16,
    age: std::time::Duration,
) -> torrust_actix::tracker::structs::torrent_peer::TorrentPeer {
    use torrust_actix::common::structs::number_of_bytes::NumberOfBytes;
    let mut peer = create_test_peer(peer_id, ip, port);
    peer.left = NumberOfBytes(0);
    peer.updated = saturating_instant_sub(std::time::Instant::now(), age);
    peer
}

/// Percent-encodes raw bytes for use in query strings (e.g. info_hash, peer_id).
pub fn percent_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("%{:02x}", b)).collect()
}