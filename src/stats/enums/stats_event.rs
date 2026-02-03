//! Statistics event types for tracking various metrics.

use serde::{Deserialize, Serialize};

/// Enumeration of all trackable statistics events.
///
/// Each variant represents a specific metric that can be incremented
/// or set. Used with `TorrentTracker::update_stats()` to update counters.
///
/// # Categories
///
/// - **Core Metrics**: Torrents, Seeds, Peers, Completed
/// - **User Metrics**: Users, UsersUpdates
/// - **Feature Metrics**: Whitelist, Blacklist, Keys
/// - **TCP IPv4**: Tcp4* variants
/// - **TCP IPv6**: Tcp6* variants
/// - **UDP IPv4**: Udp4* variants
/// - **UDP IPv6**: Udp6* variants
/// - **WebSocket**: Ws* variants
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::stats::enums::stats_event::StatsEvent;
///
/// // Increment announce counter
/// tracker.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum StatsEvent {
    Torrents,
    TorrentsUpdates,
    Users,
    UsersUpdates,
    TimestampSave,
    TimestampTimeout,
    TimestampConsole,
    TimestampKeysTimeout,
    Seeds,
    Peers,
    Completed,
    WhitelistEnabled,
    Whitelist,
    WhitelistUpdates,
    BlacklistEnabled,
    Blacklist,
    BlacklistUpdates,
    Key,
    KeyUpdates,
    Tcp4NotFound,
    Tcp4Failure,
    Tcp4ConnectionsHandled,
    Tcp4ApiHandled,
    Tcp4AnnouncesHandled,
    Tcp4ScrapesHandled,
    Tcp6NotFound,
    Tcp6Failure,
    Tcp6ConnectionsHandled,
    Tcp6ApiHandled,
    Tcp6AnnouncesHandled,
    Tcp6ScrapesHandled,
    Udp4BadRequest,
    Udp4InvalidRequest,
    Udp4ConnectionsHandled,
    Udp4AnnouncesHandled,
    Udp4ScrapesHandled,
    Udp6BadRequest,
    Udp6InvalidRequest,
    Udp6ConnectionsHandled,
    Udp6AnnouncesHandled,
    Udp6ScrapesHandled,
    UdpQueueLen,
    WsConnectionsActive,
    WsRequestsSent,
    WsRequestsReceived,
    WsResponsesSent,
    WsResponsesReceived,
    WsTimeouts,
    WsReconnects,
    WsAuthSuccess,
    WsAuthFailed,
}