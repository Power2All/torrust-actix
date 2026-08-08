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
pub struct CompressedBytes(pub(crate) Arc<[u8]>);

impl CompressedBytes {
    /// Borrows the stored bytes, still in their compressed form.
    ///
    /// Use [`CompressedBytes::decompress`] for the original string. The field itself is crate
    /// private: sharing an [`Arc`] makes the buffer cheap to clone, not immutable, and a public
    /// tuple field would invite callers to build values that skip [`CompressedBytes::compress`]
    /// and so disagree with the process-wide compression settings that `decompress` reads.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}