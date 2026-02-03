//! BitTorrent announce event types.

use serde::Deserialize;

/// The type of event in a tracker announce request.
///
/// Announce events indicate the state change of a peer as defined in BEP 3.
/// The event parameter is optional in announce requests; if omitted, it
/// indicates a regular periodic update.
///
/// # Event Types
///
/// - **None (0)**: Regular update, sent periodically while active
/// - **Completed (1)**: Sent when download finishes (becomes a seeder)
/// - **Started (2)**: Sent when beginning to download a torrent
/// - **Stopped (3)**: Sent when removing the torrent from the client
///
/// # Protocol Behavior
///
/// - `Started`: First announce when beginning a download
/// - `Completed`: Sent exactly once when download completes (left=0 for first time)
/// - `Stopped`: Final announce when closing the torrent, removes peer from swarm
/// - `None`: Regular updates during active downloading/seeding
///
/// # Example
///
/// ```rust
/// use torrust_actix::tracker::enums::announce_event::AnnounceEvent;
///
/// let event = AnnounceEvent::Completed;
/// assert_eq!(event as i32, 1);
/// ```
#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AnnounceEvent {
    /// Regular periodic update (no specific event).
    None = 0,

    /// Download completed, peer became a seeder.
    Completed = 1,

    /// Torrent download started.
    Started = 2,

    /// Torrent removed from client, peer leaving swarm.
    Stopped = 3,
}