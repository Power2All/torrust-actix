use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::structs::cluster_request::ClusterRequest;
use std::net::IpAddr;

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