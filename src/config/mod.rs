//! Configuration management module.
//!
//! This module handles loading, parsing, and validating the tracker configuration
//! from TOML files. It provides comprehensive configuration options for all
//! tracker subsystems.
//!
//! # Configuration Structure
//!
//! The main configuration file (`config.toml`) contains sections for:
//! - **tracker_config**: Core tracker settings (whitelist, blacklist, keys, users)
//! - **database**: Database connection and schema settings
//! - **cache**: Optional Redis/Memcache configuration
//! - **http_trackers**: HTTP/HTTPS server instances
//! - **udp_trackers**: UDP server instances
//! - **api_trackers**: REST API server instances
//! - **cluster**: WebSocket clustering settings (master/slave mode)
//! - **sentry**: Error reporting configuration
//!
//! # Features
//!
//! - TOML file parsing with detailed error messages
//! - Environment variable overrides
//! - Customizable database table/column names
//! - Multiple server instance configurations
//! - Default value generation
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::config::structs::configuration::Configuration;
//!
//! // Load configuration from file
//! let config = Configuration::load_from_file("config.toml").await?;
//!
//! // Generate default configuration
//! let default_config = Configuration::default();
//! Configuration::save_from_config("config.toml", &default_config)?;
//! ```

/// Configuration enumerations (cluster mode, database drivers, etc.).
pub mod enums;

/// Configuration data structures.
pub mod structs;

/// Implementation blocks for configuration loading/saving.
pub mod impls;