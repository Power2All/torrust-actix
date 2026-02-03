//! UDP tracker protocol implementation (BEP 15).
//!
//! This module implements the UDP tracker protocol as specified in:
//! - BEP 15: UDP Tracker Protocol
//! - BEP 41: UDP Tracker Protocol Extensions
//!
//! # Protocol Overview
//!
//! The UDP tracker protocol uses a connection-oriented approach:
//! 1. Client sends a connect request
//! 2. Server responds with a connection ID
//! 3. Client uses connection ID for announce/scrape requests
//!
//! # Message Types
//!
//! - **Connect** (action=0): Establish connection, get connection ID
//! - **Announce** (action=1): Register peer, get peer list
//! - **Scrape** (action=2): Query torrent statistics
//! - **Error** (action=3): Error response
//!
//! # Features
//!
//! - High-performance async UDP handling
//! - Connection ID caching
//! - IPv4 and IPv6 support (BEP 7)
//! - Proxy Protocol v2 (SPP) support for load balancers
//! - Configurable socket buffer sizes
//! - Multi-threaded packet parsing
//!
//! # Performance
//!
//! UDP is more efficient than HTTP for high-traffic trackers:
//! - Lower overhead (no HTTP headers)
//! - Faster connection establishment
//! - Better suited for millions of announces per second

/// Enumerations for UDP protocol actions and errors.
pub mod enums;

/// Implementation blocks for UDP packet handling.
pub mod impls;

/// Data structures for UDP protocol messages.
pub mod structs;

/// Traits for UDP request/response parsing.
pub mod traits;

/// Core UDP service implementation.
#[allow(clippy::module_inception)]
pub mod udp;