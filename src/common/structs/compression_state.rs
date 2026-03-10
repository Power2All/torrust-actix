use crate::config::enums::compression_algorithm::CompressionAlgorithm;

pub(crate) struct CompressionState {
    pub(crate) enabled: bool,
    pub(crate) algorithm: CompressionAlgorithm,
    pub(crate) level: u32,
}