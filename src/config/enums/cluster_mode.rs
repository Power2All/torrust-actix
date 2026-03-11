use clap::ValueEnum;
use serde::{
    Deserialize,
    Serialize
};

/// Operating mode for the tracker cluster.
///
/// Configured via `cluster` in `[tracker_config]` or the
/// `TRACKER__CLUSTER` environment variable.
///
/// # TOML values
///
/// ```toml
/// cluster = "standalone"   # or "master" / "slave"
/// ```
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum ClusterMode {
    /// Single-server mode.  No WebSocket cluster communication.
    #[default]
    standalone,
    /// Primary node.  Accepts WebSocket connections from slave nodes and
    /// broadcasts announce/scrape state changes to them.
    master,
    /// Secondary node.  Connects to the master via WebSocket and forwards
    /// all incoming requests to it for authoritative processing.
    slave,
}