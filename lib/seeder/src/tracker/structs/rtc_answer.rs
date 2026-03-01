#[derive(Debug, Clone)]
pub struct RtcAnswer {
    pub peer_id_hex: String,
    pub sdp_answer: String,
}