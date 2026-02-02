use serde::{Deserialize, Serialize};
use crate::config::enums::cluster_encoding::ClusterEncoding;

pub const CLUSTER_PROTOCOL_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeRequest {
    
    pub token: String,
    
    pub slave_id: String,
    
    pub version: u8,
}

impl HandshakeRequest {
    pub fn new(token: String, slave_id: String) -> Self {
        Self {
            token,
            slave_id,
            version: CLUSTER_PROTOCOL_VERSION,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeResponse {
    
    pub success: bool,
    
    pub error: Option<String>,
    
    pub encoding: Option<ClusterEncoding>,
    
    pub version: u8,
    
    pub master_id: Option<String>,
}

impl HandshakeResponse {
    
    pub fn success(encoding: ClusterEncoding, master_id: String) -> Self {
        Self {
            success: true,
            error: None,
            encoding: Some(encoding),
            version: CLUSTER_PROTOCOL_VERSION,
            master_id: Some(master_id),
        }
    }

    
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            encoding: None,
            version: CLUSTER_PROTOCOL_VERSION,
            master_id: None,
        }
    }
}
