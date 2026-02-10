#[cfg(test)]
mod config_tests {
    mod cluster_encoding_tests {
        use crate::config::enums::cluster_encoding::ClusterEncoding;

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

    mod cluster_mode_tests {
        use crate::config::enums::cluster_mode::ClusterMode;

        #[test]
        fn test_cluster_mode_default() {
            let mode = ClusterMode::default();
            assert_eq!(mode, ClusterMode::standalone);
        }

        #[test]
        fn test_cluster_mode_serialization() {
            let standalone_mode = ClusterMode::standalone;
            let serialized = serde_json::to_string(&standalone_mode).unwrap();
            assert_eq!(serialized, "\"standalone\"");
            let master_mode = ClusterMode::master;
            let serialized = serde_json::to_string(&master_mode).unwrap();
            assert_eq!(serialized, "\"master\"");
            let slave_mode = ClusterMode::slave;
            let serialized = serde_json::to_string(&slave_mode).unwrap();
            assert_eq!(serialized, "\"slave\"");
        }

        #[test]
        fn test_cluster_mode_deserialization() {
            let standalone_mode: ClusterMode = serde_json::from_str("\"standalone\"").unwrap();
            assert_eq!(standalone_mode, ClusterMode::standalone);
            let master_mode: ClusterMode = serde_json::from_str("\"master\"").unwrap();
            assert_eq!(master_mode, ClusterMode::master);
            let slave_mode: ClusterMode = serde_json::from_str("\"slave\"").unwrap();
            assert_eq!(slave_mode, ClusterMode::slave);
        }

        #[test]
        fn test_cluster_mode_ordering() {
            assert!(ClusterMode::standalone < ClusterMode::master);
            assert!(ClusterMode::master < ClusterMode::slave);
        }

        #[test]
        fn test_cluster_mode_equality() {
            assert_eq!(ClusterMode::standalone, ClusterMode::standalone);
            assert_ne!(ClusterMode::standalone, ClusterMode::master);
            assert_ne!(ClusterMode::master, ClusterMode::slave);
        }

        #[test]
        fn test_cluster_mode_clone() {
            let mode = ClusterMode::master;
            let cloned = mode.clone();
            assert_eq!(mode, cloned);
        }

        #[test]
        fn test_cluster_mode_debug() {
            let mode = ClusterMode::standalone;
            let debug_str = format!("{:?}", mode);
            assert_eq!(debug_str, "standalone");
        }
    }
}