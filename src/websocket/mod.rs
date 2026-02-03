//! WebSocket-based clustering module for horizontal scaling.
//!
//! This module enables running multiple tracker instances in a master/slave
//! architecture, allowing horizontal scaling for high-traffic deployments.
//!
//! # Cluster Modes
//!
//! - **Standalone**: Single instance mode (no clustering)
//! - **Master**: Authoritative node that maintains tracker state
//! - **Slave**: Forwards requests to master, serves cached responses
//!
//! # Architecture
//!
//! ```text
//!                    ┌─────────┐
//!                    │  Master │
//!                    │ Tracker │
//!                    └────┬────┘
//!             ┌───────────┼───────────┐
//!             ▼           ▼           ▼
//!        ┌────────┐  ┌────────┐  ┌────────┐
//!        │ Slave  │  │ Slave  │  │ Slave  │
//!        │   1    │  │   2    │  │   3    │
//!        └────────┘  └────────┘  └────────┘
//! ```
//!
//! # Features
//!
//! - WebSocket-based communication
//! - Multiple encoding formats (binary, JSON, MessagePack)
//! - SSL/TLS support for secure connections
//! - Automatic reconnection handling
//! - Authentication with shared secret
//! - Keep-alive with configurable intervals
//!
//! # Protocol
//!
//! Slave nodes forward announce/scrape requests to the master node,
//! which processes them and returns the response. This ensures all
//! nodes have consistent peer data.

/// Protocol type and request type enumerations.
pub mod enums;

/// Message structures for cluster communication.
pub mod structs;

/// Data encoding/decoding (binary, JSON, MessagePack).
pub mod encoding;

/// Master node server implementation.
pub mod master;

/// Slave node client implementation.
pub mod slave;