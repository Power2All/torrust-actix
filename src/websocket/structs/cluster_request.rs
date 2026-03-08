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