use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::common::structs::number_of_bytes_def::NumberOfBytesDef;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::enums::announce_event_def::AnnounceEventDef;
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
    #[serde(with = "NumberOfBytesDef")]
    pub uploaded: NumberOfBytes,
    /// Total bytes downloaded by the peer for this torrent.
    #[serde(with = "NumberOfBytesDef")]
    pub downloaded: NumberOfBytes,
    /// Bytes remaining until the download is complete (`0` for seeders).
    #[serde(with = "NumberOfBytesDef")]
    pub left: NumberOfBytes,
    /// The announce event reported by the client.
    #[serde(with = "AnnounceEventDef")]
    pub event: AnnounceEvent,
    /// WebRTC signalling state.  `None` for regular UDP/HTTP peers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_data: Option<Box<RtcData>>,
}