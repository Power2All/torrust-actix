use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClusterResponse {
    pub request_id: u64,
    pub success: bool,
    pub payload: Vec<u8>,
    pub error_message: Option<String>,
}