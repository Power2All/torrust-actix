use serde::{
    Deserialize,
    Serialize
};
use std::sync::atomic::{
    AtomicBool,
    AtomicI64
};

/// Atomic counters for all tracker metrics.
///
/// Wrapped in an [`Arc`] and shared across all request-handling threads.
/// Every field uses lock-free atomics so hot paths never block on a mutex.
/// The struct is serialisable to JSON for the `/api/stats` endpoint.
///
/// # Counter groups
///
/// | Prefix | Meaning |
/// |--------|---------|
/// | *(none)* | Global torrent / user / peer totals |
/// | `tcp4_` / `tcp6_` | IPv4 / IPv6 HTTP tracker requests |
/// | `udp4_` / `udp6_` | IPv4 / IPv6 UDP tracker requests |
/// | `ws_` | WebSocket cluster communication |
///
/// [`Arc`]: std::sync::Arc
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsAtomics {
    /// Unix timestamp (seconds) when the tracker process started.
    pub started: AtomicI64,
    /// Unix timestamp of the last database save run.
    pub timestamp_run_save: AtomicI64,
    /// Unix timestamp of the last peer-timeout cleanup run.
    pub timestamp_run_timeout: AtomicI64,
    /// Unix timestamp of the last console statistics output.
    pub timestamp_run_console: AtomicI64,
    /// Unix timestamp of the last key-timeout cleanup run.
    pub timestamp_run_keys_timeout: AtomicI64,
    /// Number of torrents currently tracked in memory.
    pub torrents: AtomicI64,
    /// Number of torrent stat changes pending the next database flush.
    pub torrents_updates: AtomicI64,
    /// Number of registered users in memory.
    pub users: AtomicI64,
    /// Number of user stat changes pending the next database flush.
    pub users_updates: AtomicI64,
    /// Total number of active seeders across all torrents.
    pub seeds: AtomicI64,
    /// Total number of active leechers across all torrents.
    pub peers: AtomicI64,
    /// Cumulative number of completed downloads recorded by the tracker.
    pub completed: AtomicI64,
    /// Whether the whitelist filter is currently active.
    pub whitelist_enabled: AtomicBool,
    /// Number of info-hashes on the whitelist.
    pub whitelist: AtomicI64,
    /// Number of pending whitelist changes.
    pub whitelist_updates: AtomicI64,
    /// Whether the blacklist filter is currently active.
    pub blacklist_enabled: AtomicBool,
    /// Number of info-hashes on the blacklist.
    pub blacklist: AtomicI64,
    /// Number of pending blacklist changes.
    pub blacklist_updates: AtomicI64,
    /// Whether the keys filter is currently active.
    pub keys_enabled: AtomicBool,
    /// Number of active announce keys.
    pub keys: AtomicI64,
    /// Number of pending key changes.
    pub keys_updates: AtomicI64,
    /// IPv4 HTTP requests that returned a 404 response.
    pub tcp4_not_found: AtomicI64,
    /// IPv4 HTTP requests that resulted in a tracker error response.
    pub tcp4_failure: AtomicI64,
    /// Total IPv4 HTTP connections handled.
    pub tcp4_connections_handled: AtomicI64,
    /// IPv4 HTTP API requests handled.
    pub tcp4_api_handled: AtomicI64,
    /// IPv4 HTTP announce requests handled.
    pub tcp4_announces_handled: AtomicI64,
    /// IPv4 HTTP scrape requests handled.
    pub tcp4_scrapes_handled: AtomicI64,
    /// IPv6 HTTP requests that returned a 404 response.
    pub tcp6_not_found: AtomicI64,
    /// IPv6 HTTP requests that resulted in a tracker error response.
    pub tcp6_failure: AtomicI64,
    /// Total IPv6 HTTP connections handled.
    pub tcp6_connections_handled: AtomicI64,
    /// IPv6 HTTP API requests handled.
    pub tcp6_api_handled: AtomicI64,
    /// IPv6 HTTP announce requests handled.
    pub tcp6_announces_handled: AtomicI64,
    /// IPv6 HTTP scrape requests handled.
    pub tcp6_scrapes_handled: AtomicI64,
    /// IPv4 UDP packets rejected as malformed.
    pub udp4_bad_request: AtomicI64,
    /// IPv4 UDP packets rejected as invalid (e.g. wrong connection ID).
    pub udp4_invalid_request: AtomicI64,
    /// Total IPv4 UDP connections handled.
    pub udp4_connections_handled: AtomicI64,
    /// IPv4 UDP announce requests handled.
    pub udp4_announces_handled: AtomicI64,
    /// IPv4 UDP scrape requests handled.
    pub udp4_scrapes_handled: AtomicI64,
    /// IPv6 UDP packets rejected as malformed.
    pub udp6_bad_request: AtomicI64,
    /// IPv6 UDP packets rejected as invalid.
    pub udp6_invalid_request: AtomicI64,
    /// Total IPv6 UDP connections handled.
    pub udp6_connections_handled: AtomicI64,
    /// IPv6 UDP announce requests handled.
    pub udp6_announces_handled: AtomicI64,
    /// IPv6 UDP scrape requests handled.
    pub udp6_scrapes_handled: AtomicI64,
    /// Current length of the UDP packet processing queue.
    pub udp_queue_len: AtomicI64,
    /// Number of active WebSocket cluster connections.
    pub ws_connections_active: AtomicI64,
    /// Cumulative WebSocket messages sent to peer nodes.
    pub ws_requests_sent: AtomicI64,
    /// Cumulative WebSocket messages received from peer nodes.
    pub ws_requests_received: AtomicI64,
    /// Cumulative WebSocket response messages sent.
    pub ws_responses_sent: AtomicI64,
    /// Cumulative WebSocket response messages received.
    pub ws_responses_received: AtomicI64,
    /// Number of WebSocket connections that timed out.
    pub ws_timeouts: AtomicI64,
    /// Number of WebSocket reconnect attempts made by slave nodes.
    pub ws_reconnects: AtomicI64,
    /// Successful WebSocket cluster authentication handshakes.
    pub ws_auth_success: AtomicI64,
    /// Failed WebSocket cluster authentication attempts.
    pub ws_auth_failed: AtomicI64,
}