//! Announce request query parameters.

use std::net::IpAddr;
use serde::Deserialize;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;

/// Parsed announce request parameters.
///
/// This struct represents the query parameters from a BitTorrent tracker
/// announce request. These parameters are defined in BEP 3 (The BitTorrent
/// Protocol Specification).
///
/// # Required Parameters (BEP 3)
///
/// - `info_hash`: 20-byte SHA-1 hash of the torrent info dictionary
/// - `peer_id`: 20-byte unique identifier for the client
/// - `port`: Port number the client is listening on
/// - `uploaded`: Total bytes uploaded since the "started" event
/// - `downloaded`: Total bytes downloaded since the "started" event
/// - `left`: Bytes remaining to download (0 = complete)
///
/// # Optional Parameters
///
/// - `compact`: If true, return peers in compact binary format (BEP 23)
/// - `no_peer_id`: If true, omit peer IDs from response
/// - `event`: One of "started", "completed", "stopped", or empty
/// - `numwant`: Number of peers the client wants (default: 50)
///
/// # Example Request
///
/// ```text
/// GET /announce?info_hash=%xx...&peer_id=%xx...&port=6881&uploaded=0&downloaded=0&left=1000000
/// ```
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct AnnounceQueryRequest {
    /// The 20-byte torrent info hash.
    pub(crate) info_hash: InfoHash,

    /// The 20-byte client peer ID.
    pub(crate) peer_id: PeerId,

    /// The port number the client is listening on.
    pub(crate) port: u16,

    /// Total bytes uploaded for this torrent.
    pub(crate) uploaded: u64,

    /// Total bytes downloaded for this torrent.
    pub(crate) downloaded: u64,

    /// Bytes remaining to download (0 = seeding).
    pub(crate) left: u64,

    /// Request compact peer list format (BEP 23).
    pub(crate) compact: bool,

    /// Omit peer IDs from the response.
    pub(crate) no_peer_id: bool,

    /// The announce event type (started, stopped, completed, or none).
    pub(crate) event: AnnounceEvent,

    /// The client's IP address (from connection or X-Real-IP header).
    pub(crate) remote_addr: IpAddr,

    /// Number of peers requested (default: 50).
    pub(crate) numwant: u64,
}