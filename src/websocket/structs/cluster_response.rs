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
