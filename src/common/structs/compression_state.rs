use crate::config::enums::compression_algorithm::CompressionAlgorithm;

/// Runtime compression settings read from the tracker configuration.
///
/// Stored in the process-wide [`COMPRESSION`] `OnceLock` and consulted by
/// every [`CompressedBytes`] compress/decompress call.
///
/// [`COMPRESSION`]: crate::common::structs::compressed_bytes::COMPRESSION
/// [`CompressedBytes`]: crate::common::structs::compressed_bytes::CompressedBytes
pub(crate) struct CompressionState {
    /// Whether compression is active.  When `false`, data is stored verbatim.
    pub(crate) enabled: bool,
    /// The compression algorithm to use.
    pub(crate) algorithm: CompressionAlgorithm,
    /// Compression level (algorithm-dependent; LZ4 ignores this value).
    pub(crate) level: u32,
}