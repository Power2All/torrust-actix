use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::websocket::types::PendingRequestSender;
use std::collections::HashMap;

pub struct SlaveClientState {
    pub encoding: Option<ClusterEncoding>,
    pub connected: bool,
    pub pending_requests: HashMap<u64, PendingRequestSender>,
    pub request_counter: u64,
}