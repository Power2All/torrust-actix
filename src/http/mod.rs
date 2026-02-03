//! HTTP/HTTPS tracker protocol implementation.
//!
//! This module implements the BitTorrent tracker protocol over HTTP/HTTPS
//! as specified in BEP 3 (The BitTorrent Protocol Specification) and
//! BEP 23 (Tracker Returns Compact Peer Lists).
//!
//! # Supported Endpoints
//!
//! - `/announce` - Handle peer announcements
//! - `/announce/{key}` - Announce with API key authentication
//! - `/announce/{key}/{passkey}` - Announce with user passkey (private tracker)
//! - `/scrape` - Query torrent statistics
//! - `/scrape/{key}` - Scrape with API key authentication
//!
//! # Features
//!
//! - Multiple concurrent HTTP/HTTPS server instances
//! - Compact peer list responses (BEP 23)
//! - IPv4 and IPv6 peer support (BEP 7)
//! - API key and user passkey authentication
//! - X-Real-IP header support for proxied requests
//! - Configurable SSL/TLS with hot-reload support
//!
//! # Response Format
//!
//! Responses are bencoded dictionaries as per the BitTorrent specification.

/// Enumerations for HTTP tracker operations.
pub mod enums;

/// Data structures for HTTP request/response handling.
pub mod structs;

/// Implementation blocks for HTTP service components.
pub mod impls;

/// Type aliases for HTTP module.
pub mod types;

/// Core HTTP service implementation.
#[allow(clippy::module_inception)]
pub mod http;