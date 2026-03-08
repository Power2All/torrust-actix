use crate::config::enums::cluster_encoding::ClusterEncoding;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HandshakeResponse {
    pub success: bool,
    pub error: Option<String>,
    pub encoding: Option<ClusterEncoding>,
    pub version: u8,
    pub master_id: Option<String>,
}