use crate::tracker::structs::rtc_answer::RtcAnswer;

#[derive(Debug, Clone, Default)]
pub struct AnnounceResponse {
    pub interval: u64,
    pub rtc_interval: Option<u64>,
    pub rtc_answers: Vec<RtcAnswer>,
    pub failure_reason: Option<String>,
}