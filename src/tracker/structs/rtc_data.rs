use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::tracker::structs::peer_id::PeerId;
use serde::{
    Deserialize,
    Serialize
};

/// WebRTC signalling state for an RtcTorrent peer.
///
/// SDP offer and answer strings are stored as [`CompressedBytes`] (LZ4 or Zstd
/// depending on configuration) to reduce memory usage.  Raw strings are
/// produced on demand by [`CompressedBytes::decompress`].
///
/// The pending-answers queue holds SDP answers submitted by leechers that
/// have not yet been collected by the originating seeder.
///
/// [`CompressedBytes`]: crate::common::structs::compressed_bytes::CompressedBytes
/// [`CompressedBytes::decompress`]: crate::common::structs::compressed_bytes::CompressedBytes::decompress
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct RtcData {
    /// The seeder's SDP offer, compressed in memory.
    pub sdp_offer: Option<CompressedBytes>,
    /// The accepted SDP answer from a leecher, compressed in memory.
    pub sdp_answer: Option<CompressedBytes>,
    /// Human-readable connection state (e.g. `"pending"`, `"connected"`).
    pub connection_status: String,
    /// Queue of `(leecher_peer_id, compressed_sdp_answer)` pairs waiting to
    /// be delivered to the seeder on its next announce poll.
    pub pending_answers: Vec<(PeerId, CompressedBytes)>,
}