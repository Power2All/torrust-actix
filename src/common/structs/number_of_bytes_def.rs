use serde::{Deserialize, Serialize};
use crate::common::structs::number_of_bytes::NumberOfBytes;

#[derive(Serialize, Deserialize)]
#[serde(remote = "NumberOfBytes")]
pub struct NumberOfBytesDef(pub i64);
