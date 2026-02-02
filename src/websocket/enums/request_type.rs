use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RequestType {
    Announce,
    Scrape,
    ApiCall {
        endpoint: String,
        method: String,
    },
    UdpPacket,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_type_announce() {
        let announce = RequestType::Announce;
        let serialized = serde_json::to_string(&announce).unwrap();
        assert_eq!(serialized, "\"Announce\"");
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RequestType::Announce);
    }

    #[test]
    fn test_request_type_scrape() {
        let scrape = RequestType::Scrape;
        let serialized = serde_json::to_string(&scrape).unwrap();
        assert_eq!(serialized, "\"Scrape\"");
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RequestType::Scrape);
    }

    #[test]
    fn test_request_type_api_call() {
        let api_call = RequestType::ApiCall {
            endpoint: "/api/v1/stats".to_string(),
            method: "GET".to_string(),
        };
        let serialized = serde_json::to_string(&api_call).unwrap();
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            RequestType::ApiCall { endpoint, method } => {
                assert_eq!(endpoint, "/api/v1/stats");
                assert_eq!(method, "GET");
            }
            _ => panic!("Expected ApiCall variant"),
        }
    }

    #[test]
    fn test_request_type_udp_packet() {
        let udp = RequestType::UdpPacket;
        let serialized = serde_json::to_string(&udp).unwrap();
        assert_eq!(serialized, "\"UdpPacket\"");
        let deserialized: RequestType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RequestType::UdpPacket);
    }

    #[test]
    fn test_request_type_equality() {
        assert_eq!(RequestType::Announce, RequestType::Announce);
        assert_ne!(RequestType::Announce, RequestType::Scrape);
        let api1 = RequestType::ApiCall {
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
        };
        let api2 = RequestType::ApiCall {
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
        };
        let api3 = RequestType::ApiCall {
            endpoint: "/other".to_string(),
            method: "POST".to_string(),
        };
        assert_eq!(api1, api2);
        assert_ne!(api1, api3);
    }

    #[test]
    fn test_request_type_clone() {
        let api_call = RequestType::ApiCall {
            endpoint: "/clone/test".to_string(),
            method: "DELETE".to_string(),
        };
        let cloned = api_call.clone();
        assert_eq!(api_call, cloned);
    }

    #[test]
    fn test_request_type_debug() {
        let announce = RequestType::Announce;
        let debug_str = format!("{:?}", announce);
        assert_eq!(debug_str, "Announce");
        let api_call = RequestType::ApiCall {
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
        };
        let debug_str = format!("{:?}", api_call);
        assert!(debug_str.contains("ApiCall"));
        assert!(debug_str.contains("/test"));
        assert!(debug_str.contains("GET"));
    }
}