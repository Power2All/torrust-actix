use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::slave::client::{send_request, SLAVE_CLIENT};
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::cluster_response::ClusterResponse;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug)]
pub enum ForwardError {
    NotConnected,
    Timeout,
    MasterError(String),
    ConnectionLost,
    EncodingError(String),
}

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForwardError::NotConnected => write!(f, "Not connected to master"),
            ForwardError::Timeout => write!(f, "Cluster timeout"),
            ForwardError::MasterError(msg) => write!(f, "Master error: {}", msg),
            ForwardError::ConnectionLost => write!(f, "Cluster connection lost"),
            ForwardError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for ForwardError {}

/// Forward a request to the master server
///
/// This function sends the request to the master and waits for a response.
/// If the request times out or the connection is lost, an appropriate error is returned.
pub async fn forward_request(
    tracker: &Arc<TorrentTracker>,
    protocol: ProtocolType,
    request_type: RequestType,
    client_ip: IpAddr,
    client_port: u16,
    payload: Vec<u8>,
) -> Result<ClusterResponse, ForwardError> {
    let request_id = {
        let mut state = SLAVE_CLIENT.write();
        state.next_request_id()
    };
    let request = ClusterRequest::new(
        request_id,
        protocol,
        request_type,
        client_ip,
        client_port,
        payload,
    );
    send_request(tracker, request).await
}

/// Create a bencode error response for timeout/connection issues
pub fn create_cluster_error_response(error: &ForwardError) -> Vec<u8> {
    let message = match error {
        ForwardError::NotConnected => "Cluster connection lost",
        ForwardError::Timeout => "Cluster timeout",
        ForwardError::MasterError(msg) => msg.as_str(),
        ForwardError::ConnectionLost => "Cluster connection lost",
        ForwardError::EncodingError(_) => "Cluster encoding error",
    };
    format!("d14:failure reason{}:{}e", message.len(), message).into_bytes()
}