use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::tracker::structs::peer_id::PeerId;
use serde::{
    Deserialize,
    Serialize
};

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct RtcData {
    pub sdp_offer: Option<CompressedBytes>,
    pub sdp_answer: Option<CompressedBytes>,
    pub connection_status: String,
    pub pending_answers: Vec<(PeerId, CompressedBytes)>,
}