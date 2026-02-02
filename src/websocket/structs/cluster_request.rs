use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_request_new() {
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        let request = ClusterRequest::new(
            42,
            ProtocolType::Http,
            RequestType::Announce,
            ip,
            6881,
            vec![1, 2, 3],
        );
        assert_eq!(request.request_id, 42);
        assert_eq!(request.protocol, ProtocolType::Http);
        assert_eq!(request.request_type, RequestType::Announce);
        assert_eq!(request.client_ip, ip);
        assert_eq!(request.client_port, 6881);
        assert_eq!(request.payload, vec![1, 2, 3]);
        assert!(request.timestamp > 0);
    }

    #[test]
    fn test_cluster_request_with_ipv6() {
        let ip: IpAddr = "::1".parse().unwrap();
        let request = ClusterRequest::new(
            1,
            ProtocolType::Udp,
            RequestType::Scrape,
            ip,
            8080,
            vec![],
        );
        assert_eq!(request.client_ip, ip);
        assert!(request.client_ip.is_ipv6());
    }

    #[test]
    fn test_cluster_request_with_api_call() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let request = ClusterRequest::new(
            100,
            ProtocolType::Api,
            RequestType::ApiCall {
                endpoint: "/api/v1/stats".to_string(),
                method: "GET".to_string(),
            },
            ip,
            443,
            vec![],
        );
        assert_eq!(request.protocol, ProtocolType::Api);
        match &request.request_type {
            RequestType::ApiCall { endpoint, method } => {
                assert_eq!(endpoint, "/api/v1/stats");
                assert_eq!(method, "GET");
            }
            _ => panic!("Expected ApiCall request type"),
        }
    }

    #[test]
    fn test_cluster_request_serialization() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let request = ClusterRequest::new(
            1,
            ProtocolType::Http,
            RequestType::Announce,
            ip,
            6969,
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ClusterRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(request.request_id, deserialized.request_id);
        assert_eq!(request.protocol, deserialized.protocol);
        assert_eq!(request.request_type, deserialized.request_type);
        assert_eq!(request.client_ip, deserialized.client_ip);
        assert_eq!(request.client_port, deserialized.client_port);
        assert_eq!(request.payload, deserialized.payload);
    }

    #[test]
    fn test_cluster_request_clone() {
        let ip: IpAddr = "192.168.0.1".parse().unwrap();
        let request = ClusterRequest::new(
            5,
            ProtocolType::Https,
            RequestType::UdpPacket,
            ip,
            12345,
            vec![1, 2, 3, 4, 5],
        );
        let cloned = request.clone();
        assert_eq!(request.request_id, cloned.request_id);
        assert_eq!(request.payload, cloned.payload);
    }

    #[test]
    fn test_cluster_request_empty_payload() {
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let request = ClusterRequest::new(
            0,
            ProtocolType::Udp,
            RequestType::Announce,
            ip,
            0,
            vec![],
        );
        assert!(request.payload.is_empty());
        assert_eq!(request.client_port, 0);
    }
}