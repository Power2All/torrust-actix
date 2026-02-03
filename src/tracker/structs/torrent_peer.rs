//! Individual peer information for a torrent.

use std::net::SocketAddr;
use serde::Serialize;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::common::structs::number_of_bytes_def::NumberOfBytesDef;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::enums::announce_event_def::AnnounceEventDef;
use crate::tracker::structs::peer_id::PeerId;

/// Information about a single peer participating in a torrent swarm.
///
/// Each `TorrentPeer` represents a BitTorrent client that has announced to the tracker.
/// It contains the peer's identification, network address, transfer statistics, and
/// the most recent announce event.
///
/// # Peer Classification
///
/// A peer is classified as a seed or leech based on the `left` field:
/// - `left == 0`: The peer is a seed (has complete copy)
/// - `left > 0`: The peer is a leech (still downloading)
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::structs::torrent_peer::TorrentPeer;
///
/// // Check if peer is a seed
/// let is_seed = peer.left.0 == 0;
///
/// // Get peer address for compact response
/// let addr = peer.peer_addr;
/// ```
#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
pub struct TorrentPeer {
    /// The unique peer identifier provided by the client.
    pub peer_id: PeerId,

    /// The network address (IP and port) where the peer can be reached.
    pub peer_addr: SocketAddr,

    /// Timestamp of the last announce from this peer.
    ///
    /// Used for peer timeout/cleanup calculations.
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant,

    /// Total bytes uploaded by this peer for this torrent.
    #[serde(with = "NumberOfBytesDef")]
    pub uploaded: NumberOfBytes,

    /// Total bytes downloaded by this peer for this torrent.
    #[serde(with = "NumberOfBytesDef")]
    pub downloaded: NumberOfBytes,

    /// Bytes remaining to download (0 = complete/seed).
    #[serde(with = "NumberOfBytesDef")]
    pub left: NumberOfBytes,

    /// The most recent announce event from this peer.
    #[serde(with = "AnnounceEventDef")]
    pub event: AnnounceEvent,
}