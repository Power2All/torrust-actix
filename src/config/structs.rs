//! Configuration data structures.
//!
//! This module contains all the struct definitions for configuration options.
//! Each struct corresponds to a section in the TOML configuration file.

/// API server configuration (address, SSL, timeouts).
pub mod api_trackers_config;

/// Cache backend configuration (Redis/Memcache).
pub mod cache_config;

/// Root configuration structure containing all settings.
pub mod configuration;

/// Database schema customization settings.
pub mod database_structure_config;

/// HTTP/HTTPS server configuration.
pub mod http_trackers_config;

/// UDP server configuration.
pub mod udp_trackers_config;

/// Blacklist table/column name customization.
pub mod database_structure_config_blacklist;

/// Keys table/column name customization.
pub mod database_structure_config_keys;

/// Torrents table/column name customization.
pub mod database_structure_config_torrents;

/// Users table/column name customization.
pub mod database_structure_config_users;

/// Whitelist table/column name customization.
pub mod database_structure_config_whitelist;

/// Database connection configuration.
pub mod database_config;

/// Core tracker settings (features, intervals, limits).
pub mod tracker_config;

/// Sentry error reporting configuration.
pub mod sentry_config;