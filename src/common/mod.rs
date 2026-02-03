//! Common utilities and shared functionality.
//!
//! This module contains helper functions and data structures used across
//! all other modules in the tracker codebase.
//!
//! # Utilities
//!
//! - Query string parsing
//! - Hex encoding/decoding
//! - IP address validation
//! - Logging setup
//! - Timestamp utilities
//! - Graceful shutdown handling
//!
//! # Data Structures
//!
//! - `CustomError` - Custom error type
//! - `NumberOfBytes` - Byte count wrapper
//! - `GetTorrentsApi` - API request struct
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::common::common::{parse_query, hex2bin, current_time};
//!
//! // Parse query string
//! let params = parse_query("info_hash=%ab%cd...&peer_id=%12%34...");
//!
//! // Convert hex string to bytes
//! let bytes = hex2bin("abcd1234...")?;
//!
//! // Get current timestamp
//! let now = current_time();
//! ```

/// Common data structures (errors, API requests, byte wrappers).
pub mod structs;

/// Core utility functions.
#[allow(clippy::module_inception)]
pub mod common;

/// Implementation blocks for common types.
pub mod impls;