use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum ClusterEncoding {
    #[default]
    binary,
    json,
    msgpack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_encoding_default() {
        let encoding = ClusterEncoding::default();
        assert_eq!(encoding, ClusterEncoding::binary);
    }

    #[test]
    fn test_cluster_encoding_serialization() {
        let binary_enc = ClusterEncoding::binary;
        let serialized = serde_json::to_string(&binary_enc).unwrap();
        assert_eq!(serialized, "\"binary\"");
        let json_enc = ClusterEncoding::json;
        let serialized = serde_json::to_string(&json_enc).unwrap();
        assert_eq!(serialized, "\"json\"");
        let msgpack_enc = ClusterEncoding::msgpack;
        let serialized = serde_json::to_string(&msgpack_enc).unwrap();
        assert_eq!(serialized, "\"msgpack\"");
    }

    #[test]
    fn test_cluster_encoding_deserialization() {
        let binary_enc: ClusterEncoding = serde_json::from_str("\"binary\"").unwrap();
        assert_eq!(binary_enc, ClusterEncoding::binary);
        let json_enc: ClusterEncoding = serde_json::from_str("\"json\"").unwrap();
        assert_eq!(json_enc, ClusterEncoding::json);
        let msgpack_enc: ClusterEncoding = serde_json::from_str("\"msgpack\"").unwrap();
        assert_eq!(msgpack_enc, ClusterEncoding::msgpack);
    }

    #[test]
    fn test_cluster_encoding_ordering() {
        assert!(ClusterEncoding::binary < ClusterEncoding::json);
        assert!(ClusterEncoding::json < ClusterEncoding::msgpack);
    }

    #[test]
    fn test_cluster_encoding_equality() {
        assert_eq!(ClusterEncoding::binary, ClusterEncoding::binary);
        assert_ne!(ClusterEncoding::binary, ClusterEncoding::json);
        assert_ne!(ClusterEncoding::json, ClusterEncoding::msgpack);
    }

    #[test]
    fn test_cluster_encoding_clone() {
        let encoding = ClusterEncoding::json;
        let cloned = encoding.clone();
        assert_eq!(encoding, cloned);
    }

    #[test]
    fn test_cluster_encoding_debug() {
        let encoding = ClusterEncoding::binary;
        let debug_str = format!("{:?}", encoding);
        assert_eq!(debug_str, "binary");
    }
}