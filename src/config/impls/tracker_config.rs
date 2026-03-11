use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::enums::cluster_mode::ClusterMode;
use std::thread::available_parallelism;

pub(crate) fn default_true() -> bool { true }
pub(crate) fn default_compression_level() -> u32 { 1 }
pub(crate) fn default_rtc_interval() -> u64 { 30 }
pub(crate) fn default_rtc_peers_timeout() -> u64 { 120 }
pub(crate) fn default_keys_cleanup_interval() -> u64 { 60 }
pub(crate) fn default_prometheus_id() -> String { String::from("torrust_actix") }
pub(crate) fn default_cluster() -> ClusterMode { ClusterMode::standalone }
pub(crate) fn default_cluster_encoding() -> ClusterEncoding { ClusterEncoding::binary }
pub(crate) fn default_cluster_bind_address() -> String { String::from("0.0.0.0:8888") }
pub(crate) fn default_cluster_keep_alive() -> u64 { 60 }
pub(crate) fn default_cluster_request_timeout() -> u64 { 15 }
pub(crate) fn default_cluster_disconnect_timeout() -> u64 { 15 }
pub(crate) fn default_cluster_reconnect_interval() -> u64 { 5 }
pub(crate) fn default_cluster_max_connections() -> u64 { 25000 }
pub(crate) fn default_cluster_threads() -> u64 { available_parallelism().map(|n| n.get() as u64).unwrap_or(4) }
pub(crate) fn default_cluster_tls_connection_rate() -> u64 { 256 }