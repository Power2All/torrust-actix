use crate::websocket::structs::cluster_response::ClusterResponse;

impl ClusterResponse {
    /// Creates a successful response carrying the payload for the given request id.
    pub fn success(request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            success: true,
            payload,
            error_message: None,
        }
    }

    /// Creates an error response carrying the failure message for the given request id.
    pub fn error(request_id: u64, error_message: String) -> Self {
        Self {
            request_id,
            success: false,
            payload: Vec::new(),
            error_message: Some(error_message),
        }
    }
}