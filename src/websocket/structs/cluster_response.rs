use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClusterResponse {
    pub request_id: u64,
    pub success: bool,
    pub payload: Vec<u8>,
    pub error_message: Option<String>,
}

impl ClusterResponse {
    pub fn success(request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            success: true,
            payload,
            error_message: None,
        }
    }

    pub fn error(request_id: u64, error_message: String) -> Self {
        Self {
            request_id,
            success: false,
            payload: Vec::new(),
            error_message: Some(error_message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_response_success() {
        let response = ClusterResponse::success(42, vec![1, 2, 3, 4]);
        assert_eq!(response.request_id, 42);
        assert!(response.success);
        assert_eq!(response.payload, vec![1, 2, 3, 4]);
        assert!(response.error_message.is_none());
    }

    #[test]
    fn test_cluster_response_error() {
        let response = ClusterResponse::error(99, "Something went wrong".to_string());
        assert_eq!(response.request_id, 99);
        assert!(!response.success);
        assert!(response.payload.is_empty());
        assert_eq!(response.error_message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_cluster_response_success_empty_payload() {
        let response = ClusterResponse::success(0, vec![]);
        assert!(response.success);
        assert!(response.payload.is_empty());
    }

    #[test]
    fn test_cluster_response_serialization() {
        let response = ClusterResponse::success(123, vec![0xca, 0xfe]);
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: ClusterResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response.request_id, deserialized.request_id);
        assert_eq!(response.success, deserialized.success);
        assert_eq!(response.payload, deserialized.payload);
        assert_eq!(response.error_message, deserialized.error_message);
    }

    #[test]
    fn test_cluster_response_error_serialization() {
        let response = ClusterResponse::error(456, "Error message".to_string());
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: ClusterResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response.request_id, deserialized.request_id);
        assert!(!deserialized.success);
        assert_eq!(response.error_message, deserialized.error_message);
    }

    #[test]
    fn test_cluster_response_clone() {
        let response = ClusterResponse::success(1, vec![10, 20, 30]);
        let cloned = response.clone();
        assert_eq!(response.request_id, cloned.request_id);
        assert_eq!(response.payload, cloned.payload);
    }

    #[test]
    fn test_cluster_response_debug() {
        let response = ClusterResponse::success(1, vec![]);
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("ClusterResponse"));
        assert!(debug_str.contains("request_id"));
        assert!(debug_str.contains("success: true"));
    }
}