use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::rtc_data::RtcData;
use serde::{
    Deserialize,
    Serialize
};
use std::net::SocketAddr;

/// A single peer participating in a torrent swarm.
///
/// Peers are stored in [`TorrentEntry`] under their [`PeerId`].  For regular
/// UDP/HTTP peers `rtc_data` is `None`, which keeps the struct small via the
/// null-pointer niche optimisation on `Box`.  RtcTorrent (WebRTC) peers carry
/// an additional heap-allocated [`RtcData`] block containing their SDP
/// signalling state.
///
/// [`TorrentEntry`]: crate::tracker::structs::torrent_entry::TorrentEntry
/// [`PeerId`]: crate::tracker::structs::peer_id::PeerId
/// [`RtcData`]: crate::tracker::structs::rtc_data::RtcData
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TorrentPeer {
    /// Unique 20-byte peer identifier supplied by the client.
    pub peer_id: PeerId,
    /// IP address and port the peer is reachable at.
    pub peer_addr: SocketAddr,
    /// Monotonic timestamp of the last announce from this peer.
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant,
    /// Total bytes uploaded by the peer for this torrent.
    pub uploaded: NumberOfBytes,
    /// Total bytes downloaded by the peer for this torrent.
    pub downloaded: NumberOfBytes,
    /// Bytes remaining until the download is complete (`0` for seeders).
    pub left: NumberOfBytes,
    /// The announce event reported by the client.
    pub event: AnnounceEvent,
    /// The announce `key`, which a client never shares with other peers: it lets a client whose IP
    /// changed prove it is the same client. `None` when the client sent none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<u32>,
    /// WebRTC signalling state.  `None` for regular UDP/HTTP peers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_data: Option<Box<RtcData>>,
}