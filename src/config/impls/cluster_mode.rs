use crate::config::enums::cluster_mode::ClusterMode;

impl ClusterMode {
    pub fn is_master(&self) -> bool {
        matches!(self, ClusterMode::master)
    }

    pub fn is_slave(&self) -> bool {
        matches!(self, ClusterMode::slave)
    }

    pub fn is_standalone(&self) -> bool {
        matches!(self, ClusterMode::standalone)
    }
}