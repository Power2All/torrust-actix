use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::tracker::structs::rtc_data::RtcData;

impl RtcData {
    /// Creates fresh RTC signalling state, compressing the SDP offer if given.
    pub fn new(sdp_offer: Option<&str>) -> Self {
        RtcData {
            sdp_offer: sdp_offer.map(CompressedBytes::compress),
            sdp_answer: None,
            pending_answers: Vec::new(),
        }
    }
}