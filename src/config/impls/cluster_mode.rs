use crate::config::enums::cluster_mode::ClusterMode;

impl ClusterMode {
    /// Returns `true` when this node runs as the cluster master.
    pub fn is_master(&self) -> bool {
        matches!(self, ClusterMode::master)
    }

    /// Returns `true` when this node runs as a cluster slave (forwards requests to the master).
    pub fn is_slave(&self) -> bool {
        matches!(self, ClusterMode::slave)
    }

    /// Returns `true` when this node runs standalone (no clustering).
    pub fn is_standalone(&self) -> bool {
        matches!(self, ClusterMode::standalone)
    }
}