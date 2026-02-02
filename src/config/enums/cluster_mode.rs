use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum ClusterMode {
    #[default]
    standalone,
    master,
    slave,
}

#[cfg(test)]
mod tests {
    use super::*;

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