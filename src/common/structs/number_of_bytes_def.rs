use crate::common::structs::number_of_bytes::NumberOfBytes;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize)]
#[serde(remote = "NumberOfBytes")]
pub struct NumberOfBytesDef(pub i64);