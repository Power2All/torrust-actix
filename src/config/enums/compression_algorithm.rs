use serde::{
    Deserialize,
    Serialize
};

/// The compression algorithm used to store RtcTorrent SDP strings in memory.
///
/// Configured via `rtc_compression_algorithm` in `[tracker_config]` or the
/// `TRACKER__RTC_COMPRESSION_ALGORITHM` environment variable.
///
/// # TOML values
///
/// ```toml
/// rtc_compression_algorithm = "lz4"   # or "zstd"
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgorithm {
    /// LZ4 — extremely fast compression with moderate ratios.  Recommended
    /// for most deployments.  The `level` setting is ignored by LZ4.
    #[default]
    Lz4,
    /// Zstd — slower but achieves better compression ratios.  Accepts levels
    /// 1–22; level 1 is fast and already compresses well for SDP data.
    Zstd,
}