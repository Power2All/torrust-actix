use crate::common::structs::compression_state::CompressionState;
use serde::{
    Deserialize,
    Serialize
};
use std::sync::OnceLock;

pub(crate) static COMPRESSION: OnceLock<CompressionState> = OnceLock::new();

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct CompressedBytes(pub Vec<u8>);