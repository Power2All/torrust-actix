use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use serde::{
    Deserialize,
    Serialize
};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClusterRequest {
    pub request_id: u64,
    pub protocol: ProtocolType,
    pub request_type: RequestType,
    pub client_ip: IpAddr,
    pub client_port: u16,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

impl ClusterRequest {
    pub fn new(
        request_id: u64,
        protocol: ProtocolType,
        request_type: RequestType,
        client_ip: IpAddr,
        client_port: u16,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            request_id,
            protocol,
            request_type,
            client_ip,
            client_port,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}