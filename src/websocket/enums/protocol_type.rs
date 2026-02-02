use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProtocolType {
    Http,
    Https,
    Udp,
    Api,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_type_serialization() {
        let http = ProtocolType::Http;
        let serialized = serde_json::to_string(&http).unwrap();
        assert_eq!(serialized, "\"Http\"");
        let https = ProtocolType::Https;
        let serialized = serde_json::to_string(&https).unwrap();
        assert_eq!(serialized, "\"Https\"");
        let udp = ProtocolType::Udp;
        let serialized = serde_json::to_string(&udp).unwrap();
        assert_eq!(serialized, "\"Udp\"");
        let api = ProtocolType::Api;
        let serialized = serde_json::to_string(&api).unwrap();
        assert_eq!(serialized, "\"Api\"");
    }

    #[test]
    fn test_protocol_type_deserialization() {
        let http: ProtocolType = serde_json::from_str("\"Http\"").unwrap();
        assert_eq!(http, ProtocolType::Http);
        let https: ProtocolType = serde_json::from_str("\"Https\"").unwrap();
        assert_eq!(https, ProtocolType::Https);
        let udp: ProtocolType = serde_json::from_str("\"Udp\"").unwrap();
        assert_eq!(udp, ProtocolType::Udp);
        let api: ProtocolType = serde_json::from_str("\"Api\"").unwrap();
        assert_eq!(api, ProtocolType::Api);
    }

    #[test]
    fn test_protocol_type_equality() {
        assert_eq!(ProtocolType::Http, ProtocolType::Http);
        assert_ne!(ProtocolType::Http, ProtocolType::Https);
        assert_ne!(ProtocolType::Udp, ProtocolType::Api);
    }

    #[test]
    fn test_protocol_type_clone() {
        let http = ProtocolType::Http;
        let cloned = http.clone();
        assert_eq!(http, cloned);
    }

    #[test]
    fn test_protocol_type_debug() {
        let http = ProtocolType::Http;
        let debug_str = format!("{:?}", http);
        assert_eq!(debug_str, "Http");
    }
}