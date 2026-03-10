use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::tracker::structs::rtc_data::RtcData;

impl RtcData {
    pub fn new(sdp_offer: Option<&str>) -> Self {
        RtcData {
            sdp_offer: sdp_offer.map(CompressedBytes::compress),
            sdp_answer: None,
            connection_status: "pending".to_string(),
            pending_answers: Vec::new(),
        }
    }
}