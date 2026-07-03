use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
use std::thread::available_parallelism;

/// Serde default: `true`.
pub(crate) fn default_true() -> bool { true }
/// Serde default compression level for in-memory SDP storage: `1`.
pub(crate) fn default_compression_level() -> u32 { 1 }
/// Serde default RtcTorrent announce interval in seconds: `30`.
pub(crate) fn default_rtc_interval() -> u64 { 30 }
/// Serde default RtcTorrent peer timeout in seconds: `120`.
pub(crate) fn default_rtc_peers_timeout() -> u64 { 120 }
/// Serde default interval in seconds between key-expiry sweeps: `60`.
pub(crate) fn default_keys_cleanup_interval() -> u64 { 60 }
/// Serde default Prometheus metric namespace: `torrust_actix`.
pub(crate) fn default_prometheus_id() -> String { String::from("torrust_actix") }
/// Serde default cluster mode: standalone (no clustering).
pub(crate) fn default_cluster() -> ClusterMode { ClusterMode::standalone }
/// Serde default cluster wire encoding: binary (MessagePack).
pub(crate) fn default_cluster_encoding() -> ClusterEncoding { ClusterEncoding::binary }
/// Serde default cluster listener address: `0.0.0.0:8888`.
pub(crate) fn default_cluster_bind_address() -> String { String::from("0.0.0.0:8888") }
/// Serde default cluster keep-alive in seconds: `60`.
pub(crate) fn default_cluster_keep_alive() -> u64 { 60 }
/// Serde default cluster request timeout in seconds: `15`.
pub(crate) fn default_cluster_request_timeout() -> u64 { 15 }
/// Serde default cluster disconnect timeout in seconds: `15`.
pub(crate) fn default_cluster_disconnect_timeout() -> u64 { 15 }
/// Serde default delay in seconds before a slave reconnects to its master: `5`.
pub(crate) fn default_cluster_reconnect_interval() -> u64 { 5 }
/// Serde default maximum concurrent cluster connections: `25000`.
pub(crate) fn default_cluster_max_connections() -> u64 { 25000 }
/// Serde default cluster worker threads: the machine's available parallelism (fallback 4).
pub(crate) fn default_cluster_threads() -> u64 { available_parallelism().map(|n| n.get() as u64).unwrap_or(4) }
/// Serde default maximum TLS handshakes per second on the cluster listener: `256`.
pub(crate) fn default_cluster_tls_connection_rate() -> u64 { 256 }

/// Serde default number of rows per database transaction chunk during syncs: `1000`.
///
/// Keeps transactions short so external writers are not blocked by long-held locks.
pub(crate) fn default_chunk_size() -> u64 { 1000 }

/// Serde default Sentry event sample rate: `1.0` (all events).
pub(crate) fn default_sample_rate() -> f32 { 1.0 }
/// Serde default Sentry tracing sample rate: `1.0` (all transactions).
pub(crate) fn default_traces_sample_rate() -> f32 { 1.0 }
/// Serde default maximum Sentry breadcrumbs kept per event: `100`.
pub(crate) fn default_max_breadcrumbs() -> usize { 100 }
/// Serde default for attaching stack traces to Sentry events: `true`.
pub(crate) fn default_attach_stacktrace() -> bool { true }