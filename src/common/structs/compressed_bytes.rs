use crate::common::structs::compression_state::CompressionState;
use serde::{
    Deserialize,
    Serialize
};
use std::sync::{
    Arc,
    OnceLock
};

/// Process-wide compression configuration, initialised once at startup.
///
/// Set by [`init_compression`] before any `CompressedBytes` are created.
///
/// [`init_compression`]: crate::common::common::init_compression
pub(crate) static COMPRESSION: OnceLock<CompressionState> = OnceLock::new();

/// A byte buffer that stores its contents in compressed form.
///
/// Compression and decompression are performed transparently via
/// [`CompressedBytes::compress`] and [`CompressedBytes::decompress`].
/// The algorithm (LZ4 or Zstd) and level are determined by the global
/// compression state initialised at startup via
/// [`init_compression`](crate::common::common::init_compression).
///
/// When compression is disabled the raw bytes are stored as-is, so callers
/// never need to handle both cases explicitly.
///
/// The buffer is behind an [`Arc`] because these values are cloned wholesale on
/// every announce (see [`AnnounceEntry::from_entry`]): a peer's SDP offer, its
/// answer and its whole pending-answer queue are copied into the response
/// snapshot. With an owned `Vec` a swarm full of RtcTorrent peers turns each
/// announce into tens of megabytes of `memcpy`; sharing makes the clone a
/// refcount bump. The contents are never mutated in place, only replaced.
///
/// [`AnnounceEntry::from_entry`]: crate::tracker::structs::announce_entry::AnnounceEntry
///
/// # Example
///
/// ```no_run
/// use torrust_actix::common::structs::compressed_bytes::CompressedBytes;
///
/// let cb = CompressedBytes::compress("v=0\r\na=...");
/// let sdp = cb.decompress();
/// assert_eq!(sdp, "v=0\r\na=...");
/// ```
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct CompressedBytes(pub Arc<[u8]>);