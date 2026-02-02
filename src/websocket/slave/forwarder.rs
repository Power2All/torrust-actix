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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_error_display_not_connected() {
        let error = ForwardError::NotConnected;
        assert_eq!(format!("{}", error), "Not connected to master");
    }

    #[test]
    fn test_forward_error_display_timeout() {
        let error = ForwardError::Timeout;
        assert_eq!(format!("{}", error), "Cluster timeout");
    }

    #[test]
    fn test_forward_error_display_master_error() {
        let error = ForwardError::MasterError("Internal server error".to_string());
        assert_eq!(format!("{}", error), "Master error: Internal server error");
    }

    #[test]
    fn test_forward_error_display_connection_lost() {
        let error = ForwardError::ConnectionLost;
        assert_eq!(format!("{}", error), "Cluster connection lost");
    }

    #[test]
    fn test_forward_error_display_encoding_error() {
        let error = ForwardError::EncodingError("Invalid msgpack".to_string());
        assert_eq!(format!("{}", error), "Encoding error: Invalid msgpack");
    }

    #[test]
    fn test_forward_error_debug() {
        let error = ForwardError::NotConnected;
        let debug_str = format!("{:?}", error);
        assert_eq!(debug_str, "NotConnected");
        let error = ForwardError::MasterError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MasterError"));
    }

    #[test]
    fn test_create_cluster_error_response_not_connected() {
        let error = ForwardError::NotConnected;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.starts_with("d14:failure reason"));
        assert!(response_str.ends_with("e"));
        assert!(response_str.contains("Cluster connection lost"));
    }

    #[test]
    fn test_create_cluster_error_response_timeout() {
        let error = ForwardError::Timeout;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Cluster timeout"));
    }

    #[test]
    fn test_create_cluster_error_response_master_error() {
        let error = ForwardError::MasterError("Custom error message".to_string());
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Custom error message"));
    }

    #[test]
    fn test_create_cluster_error_response_connection_lost() {
        let error = ForwardError::ConnectionLost;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Cluster connection lost"));
    }

    #[test]
    fn test_create_cluster_error_response_encoding_error() {
        let error = ForwardError::EncodingError("bad data".to_string());
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("Cluster encoding error"));
    }

    #[test]
    fn test_create_cluster_error_response_bencode_format() {
        let error = ForwardError::Timeout;
        let response = create_cluster_error_response(&error);
        let response_str = String::from_utf8(response).unwrap();
        assert_eq!(response_str, "d14:failure reason15:Cluster timeoute");
    }
}