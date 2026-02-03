//! # Torrust-Actix BitTorrent Tracker
//!
//! A high-performance, feature-rich BitTorrent tracker built with Rust and the Actix-web framework.
//!
//! ## Overview
//!
//! Torrust-Actix is a modern BitTorrent tracker that supports multiple protocols (HTTP/HTTPS, UDP),
//! various database backends (SQLite, MySQL, PostgreSQL), and optional caching layers (Redis, Memcache).
//! It implements the BitTorrent Enhancement Proposals (BEPs) for tracker functionality.
//!
//! ## Features
//!
//! - **Multi-Protocol Support**: HTTP/HTTPS and UDP tracker protocols
//! - **Database Agnostic**: SQLite, MySQL, and PostgreSQL support with customizable schemas
//! - **Caching Layer**: Optional Redis or Memcache caching for improved performance
//! - **Clustering**: Master/slave architecture via WebSocket for horizontal scaling
//! - **User Management**: Optional user accounts with per-user statistics tracking
//! - **Security**: Whitelist/blacklist support, API keys, and user authentication keys
//! - **SSL/TLS**: Hot-reloadable SSL certificates without server restart
//! - **Monitoring**: Real-time statistics, Prometheus metrics, and Sentry integration
//!
//! ## BEP Compliance
//!
//! This tracker implements the following BitTorrent Enhancement Proposals:
//! - BEP 3: The BitTorrent Protocol Specification
//! - BEP 7: IPv6 Tracker Extension
//! - BEP 15: UDP Tracker Protocol
//! - BEP 23: Tracker Returns Compact Peer Lists
//! - BEP 41: UDP Tracker Protocol Extensions
//! - BEP 48: Tracker Protocol Extension: Scrape
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use torrust_actix::config::Configuration;
//! use torrust_actix::tracker::TorrentTracker;
//!
//! // Load configuration from file
//! let config = Configuration::load_from_file("config.toml").await?;
//!
//! // Create tracker instance
//! let tracker = TorrentTracker::new(config).await?;
//! ```
//!
//! ## Modules
//!
//! - [`api`] - REST API endpoints for tracker management and statistics
//! - [`cache`] - Redis and Memcache caching implementations
//! - [`common`] - Shared utilities, error handling, and helper functions
//! - [`config`] - Configuration management and TOML parsing
//! - [`database`] - Multi-database backend support (SQLite, MySQL, PostgreSQL)
//! - [`http`] - HTTP/HTTPS tracker protocol implementation
//! - [`ssl`] - SSL/TLS certificate management with hot-reload support
//! - [`stats`] - Real-time statistics tracking and monitoring
//! - [`structs`] - CLI argument parsing and common data structures
//! - [`tracker`] - Core tracker logic, peer management, and torrent handling
//! - [`udp`] - UDP tracker protocol implementation (BEP 15)
//! - [`websocket`] - WebSocket-based clustering for master/slave architecture

/// REST API module for tracker management and statistics.
///
/// Provides HTTP endpoints for managing torrents, users, whitelists, blacklists,
/// API keys, and retrieving tracker statistics. Includes Swagger UI documentation
/// and Prometheus metrics endpoints.
pub mod api;

/// Caching layer module supporting Redis and Memcache.
///
/// Implements peer data caching to reduce database load and improve response times
/// for high-traffic tracker deployments.
pub mod cache;

/// Common utilities and shared functionality.
///
/// Contains helper functions for query parsing, IP validation, hex conversion,
/// logging setup, and error handling used across all modules.
pub mod common;

/// Configuration management module.
///
/// Handles loading, parsing, and validating configuration from TOML files
/// and environment variables. Supports customizable database schemas and
/// multi-server configurations.
pub mod config;

/// Database backend module with multi-database support.
///
/// Provides a unified interface for SQLite, MySQL, and PostgreSQL backends
/// with support for custom table and column names. Includes query builders
/// and connection pooling.
pub mod database;

/// HTTP/HTTPS tracker protocol implementation.
///
/// Handles announce and scrape requests over HTTP/HTTPS according to the
/// BitTorrent tracker protocol specification. Supports multiple concurrent
/// server instances with configurable SSL/TLS.
pub mod http;

/// SSL/TLS certificate management module.
///
/// Provides certificate storage, hot-reloading capabilities, and dynamic
/// certificate resolution for SNI-based virtual hosting.
pub mod ssl;

/// Statistics tracking and monitoring module.
///
/// Collects real-time metrics on tracker activity including peer counts,
/// announce/scrape requests, protocol-specific statistics, and cluster activity.
/// Supports Prometheus metrics export.
pub mod stats;

/// CLI argument parsing and common data structures.
///
/// Defines command-line interface options for the tracker binary including
/// configuration generation, database setup, and data import/export.
pub mod structs;

/// Core tracker logic module.
///
/// Contains the main tracker implementation including peer management,
/// torrent tracking, user accounts, sharding for scalability, and the
/// announce/scrape request handling logic.
pub mod tracker;

/// UDP tracker protocol implementation (BEP 15).
///
/// Implements the UDP tracker protocol with support for connect, announce,
/// and scrape actions. Includes proxy protocol support for load balancer
/// compatibility.
pub mod udp;

/// WebSocket-based clustering module.
///
/// Enables horizontal scaling through master/slave architecture. The master
/// node maintains the authoritative tracker state while slave nodes forward
/// requests and serve cached responses.
pub mod websocket;