use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
use crate::config::enums::compression_algorithm::CompressionAlgorithm;
use serde::{
    Deserialize,
    Serialize
};

/// Core tracker behaviour settings (`[tracker_config]` in `config.toml`).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackerConfig {
    /// API authentication key required for all `/api/` requests.
    pub api_key: String,
    /// When `true`, only info-hashes on the whitelist are tracked.
    pub whitelist_enabled: bool,
    /// When `true`, info-hashes on the blacklist are rejected.
    pub blacklist_enabled: bool,
    /// When `true`, announces must include a valid access key.
    pub keys_enabled: bool,
    /// Interval in seconds between expired-key cleanup runs.
    pub keys_cleanup_interval: u64,
    /// When `true`, per-user statistics are tracked.
    pub users_enabled: bool,
    /// Suggested re-announce interval returned to clients (seconds).
    pub request_interval: u64,
    /// Minimum re-announce interval enforced server-side (seconds).
    pub request_interval_minimum: u64,
    /// Seconds of inactivity after which a peer is considered timed out.
    pub peers_timeout: u64,
    /// Interval in seconds between peer-timeout cleanup runs.
    pub peers_cleanup_interval: u64,
    /// Number of parallel threads used for peer cleanup.
    pub peers_cleanup_threads: u64,
    /// Cumulative download count loaded from (or persisted to) the database.
    pub total_downloads: u64,
    /// Enable the built-in Swagger UI at `<api>/swagger-ui/`.
    pub swagger: bool,
    /// Identifier label attached to Prometheus metrics.
    pub prometheus_id: String,
    /// Cluster operating mode.
    pub cluster: ClusterMode,
    /// Wire encoding used for cluster WebSocket messages.
    pub cluster_encoding: ClusterEncoding,
    /// Shared secret token used to authenticate cluster connections.
    pub cluster_token: String,
    /// Address the master node listens on for slave connections.
    pub cluster_bind_address: String,
    /// Address of the master node (used by slave nodes).
    pub cluster_master_address: String,
    /// WebSocket keep-alive interval in seconds.
    pub cluster_keep_alive: u64,
    /// Timeout in seconds for cluster request/response round trips.
    pub cluster_request_timeout: u64,
    /// Timeout in seconds before a silent cluster connection is closed.
    pub cluster_disconnect_timeout: u64,
    /// Interval in seconds between slave reconnect attempts.
    pub cluster_reconnect_interval: u64,
    /// Maximum simultaneous slave connections accepted by the master.
    pub cluster_max_connections: u64,
    /// Worker threads dedicated to cluster WebSocket I/O.
    pub cluster_threads: u64,
    /// Enable TLS for cluster WebSocket connections.
    pub cluster_ssl: bool,
    /// Path to the TLS private key file for cluster connections.
    pub cluster_ssl_key: String,
    /// Path to the TLS certificate file for cluster connections.
    pub cluster_ssl_cert: String,
    /// Maximum new TLS cluster connections accepted per second.
    pub cluster_tls_connection_rate: u64,
    /// Interval in seconds between RtcTorrent peer-state polls.
    pub rtc_interval: u64,
    /// Seconds of inactivity after which an RtcTorrent peer is removed.
    pub rtc_peers_timeout: u64,
    /// Enable in-memory compression for RTC SDP offer/answer strings.
    /// Defaults to `true`; omit from `config.toml` to keep the default.
    #[serde(default = "crate::config::impls::tracker_config::default_true")]
    pub rtc_compression_enabled: bool,
    /// Compression algorithm for RTC SDP data.
    /// Defaults to [`CompressionAlgorithm::Lz4`]; omit to keep the default.
    #[serde(default)]
    pub rtc_compression_algorithm: CompressionAlgorithm,
    /// Compression level (Zstd: 1–22; LZ4: ignored).
    /// Defaults to `1`; omit from `config.toml` to keep the default.
    #[serde(default = "crate::config::impls::tracker_config::default_compression_level")]
    pub rtc_compression_level: u32,
}