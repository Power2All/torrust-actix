use crate::tracker::structs::rtc_answer::RtcAnswer;

#[derive(Debug, Clone, Default)]
pub struct RtcAnnounceResponse {
    pub interval: u64,
    pub rtc_interval: Option<u64>,
    pub rtc_answers: Vec<RtcAnswer>,
    pub failure_reason: Option<String>,
}

#[derive(Clone)]
pub struct RtcTrackerClient {
    pub tracker_url: String,
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub http: reqwest::Client,
}