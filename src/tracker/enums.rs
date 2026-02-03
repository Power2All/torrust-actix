//! Enumerations for tracker operations.
//!
//! This module contains enum definitions for various tracker states and actions,
//! including announce events, peer filtering types, and database update actions.

/// Announce event types from BitTorrent protocol.
///
/// Represents the event parameter in announce requests:
/// - `None` - Regular update
/// - `Started` - New download started
/// - `Stopped` - Download stopped
/// - `Completed` - Download completed (became a seeder)
pub mod announce_event;

/// Serde serialization definition for AnnounceEvent.
///
/// Used with `#[serde(with = "...")]` attribute for custom serialization
/// of announce events.
pub mod announce_event_def;

/// Peer filtering types for announce responses.
///
/// Used to specify which peers to return:
/// - `All` - Return both IPv4 and IPv6 peers
/// - `IPv4` - Return only IPv4 peers
/// - `IPv6` - Return only IPv6 peers
pub mod torrent_peers_type;

/// Database update action types.
///
/// Specifies the type of pending database operation:
/// - `Add` - Insert new record
/// - `Update` - Modify existing record
/// - `Remove` - Delete record
pub mod updates_action;