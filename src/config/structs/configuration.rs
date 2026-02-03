//! Root configuration structure.

use serde::{Deserialize, Serialize};
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::cache_config::CacheConfig;
use crate::config::structs::database_config::DatabaseConfig;
use crate::config::structs::database_structure_config::DatabaseStructureConfig;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::config::structs::sentry_config::SentryConfig;
use crate::config::structs::tracker_config::TrackerConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;

/// The root configuration structure for the tracker.
///
/// This struct represents the complete configuration loaded from `config.toml`.
/// It contains all settings organized into logical sections.
///
/// # Configuration File Structure
///
/// ```toml
/// log_level = "info"
/// log_console_interval = 60
///
/// [tracker_config]
/// # Core tracker settings...
///
/// [sentry_config]
/// # Error reporting settings...
///
/// [database]
/// # Database connection settings...
///
/// [database_structure]
/// # Custom table/column names...
///
/// [cache]
/// # Optional cache settings...
///
/// [[http_server]]
/// # HTTP server instance...
///
/// [[udp_server]]
/// # UDP server instance...
///
/// [[api_server]]
/// # API server instance...
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::config::structs::configuration::Configuration;
///
/// let config = Configuration::load_from_file("config.toml").await?;
/// println!("Log level: {}", config.log_level);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    /// Logging level (trace, debug, info, warn, error).
    pub log_level: String,

    /// Interval in seconds between console statistics output.
    pub log_console_interval: u64,

    /// Core tracker configuration (features, intervals, limits).
    pub tracker_config: TrackerConfig,

    /// Sentry error reporting configuration.
    pub sentry_config: SentryConfig,

    /// Database connection configuration.
    pub database: DatabaseConfig,

    /// Custom database table/column names.
    pub database_structure: DatabaseStructureConfig,

    /// Optional cache backend configuration.
    #[serde(default)]
    pub cache: Option<CacheConfig>,

    /// List of HTTP/HTTPS server configurations.
    pub http_server: Vec<HttpTrackersConfig>,

    /// List of UDP server configurations.
    pub udp_server: Vec<UdpTrackersConfig>,

    /// List of REST API server configurations.
    pub api_server: Vec<ApiTrackersConfig>,
}