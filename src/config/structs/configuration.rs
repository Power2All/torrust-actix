use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::cache_config::CacheConfig;
use crate::config::structs::database_config::DatabaseConfig;
use crate::config::structs::database_structure_config::DatabaseStructureConfig;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::config::structs::sentry_config::SentryConfig;
use crate::config::structs::tracker_config::TrackerConfig;
use crate::config::structs::udp_trackers_config::UdpTrackersConfig;
use serde::{
    Deserialize,
    Serialize
};

/// Root application configuration, deserialised from `config.toml`.
///
/// Generate a default file with:
/// ```bash
/// torrust-actix --create-config
/// ```
///
/// All top-level fields map directly to TOML sections.  Most can also be
/// overridden at runtime via environment variables — see the README for the
/// full list.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    /// Logging verbosity: `off | trace | debug | info | warn | error`.
    pub log_level: String,
    /// Interval in seconds between console statistics lines.
    pub log_console_interval: u64,
    /// Core tracker behaviour (announce intervals, peer timeouts, cluster, …).
    pub tracker_config: TrackerConfig,
    /// Sentry error-tracking integration settings.
    pub sentry_config: SentryConfig,
    /// Database connection and persistence settings.
    pub database: DatabaseConfig,
    /// Column/table name overrides for the database schema.
    pub database_structure: DatabaseStructureConfig,
    /// Optional Redis or Memcache peer-data cache.  Omit section to disable.
    #[serde(default)]
    pub cache: Option<CacheConfig>,
    /// One or more HTTP/HTTPS tracker listeners.
    pub http_server: Vec<HttpTrackersConfig>,
    /// One or more UDP tracker listeners.
    pub udp_server: Vec<UdpTrackersConfig>,
    /// One or more HTTP/HTTPS API listeners.
    pub api_server: Vec<ApiTrackersConfig>,
}