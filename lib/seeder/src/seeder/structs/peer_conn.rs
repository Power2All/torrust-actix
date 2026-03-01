use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;

pub struct PeerConn {
    pub peer_connection: Arc<RTCPeerConnection>,
    #[allow(dead_code)]
    pub data_channel: Arc<RTCDataChannel>,
    pub sdp_offer: String,
}