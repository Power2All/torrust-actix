use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::common::structs::compressed_bytes::COMPRESSION;
use crate::config::enums::compression_algorithm::CompressionAlgorithm;

impl CompressedBytes {
    pub fn compress(s: &str) -> Self {
        let state = COMPRESSION.get();
        let enabled = state.is_some_and(|st| st.enabled);
        if !enabled || s.is_empty() {
            return CompressedBytes(s.as_bytes().to_vec());
        }
        let level = state.map_or(1, |st| st.level);
        match state.map(|st| &st.algorithm).unwrap_or(&CompressionAlgorithm::Lz4) {
            CompressionAlgorithm::Lz4 => {
                CompressedBytes(lz4_flex::compress_prepend_size(s.as_bytes()))
            }
            CompressionAlgorithm::Zstd => {
                CompressedBytes(
                    zstd::encode_all(s.as_bytes(), level as i32)
                        .unwrap_or_else(|_| s.as_bytes().to_vec())
                )
            }
        }
    }

    pub fn decompress(&self) -> String {
        let state = COMPRESSION.get();
        let enabled = state.is_some_and(|st| st.enabled);
        if !enabled {
            return String::from_utf8_lossy(&self.0).into_owned();
        }
        match state.map(|st| &st.algorithm).unwrap_or(&CompressionAlgorithm::Lz4) {
            CompressionAlgorithm::Lz4 => {
                lz4_flex::decompress_size_prepended(&self.0)
                    .ok()
                    .and_then(|v| String::from_utf8(v).ok())
                    .unwrap_or_default()
            }
            CompressionAlgorithm::Zstd => {
                zstd::decode_all(self.0.as_slice())
                    .ok()
                    .and_then(|v| String::from_utf8(v).ok())
                    .unwrap_or_default()
            }
        }
    }
}