//! Real-time statistics tracking and monitoring module.
//!
//! This module provides atomic counters for tracking all tracker activity,
//! enabling real-time monitoring and Prometheus metrics export.
//!
//! # Statistics Categories
//!
//! ## Core Metrics
//! - Torrent counts and updates
//! - Peer counts (seeds, leeches)
//! - Completion counts
//! - User counts and updates
//!
//! ## Protocol Metrics
//! - TCP IPv4/IPv6 connections, announces, scrapes
//! - UDP IPv4/IPv6 connections, announces, scrapes
//! - API request counts
//! - Error and failure counts
//!
//! ## Feature Metrics
//! - Whitelist/blacklist entries and updates
//! - API key counts and updates
//! - WebSocket cluster activity
//!
//! # Thread Safety
//!
//! All statistics are stored as atomic integers, allowing safe concurrent
//! updates from multiple worker threads without locking overhead.
//!
//! # Monitoring Integration
//!
//! - JSON format via `/stats` endpoint
//! - Prometheus format via `/metrics` endpoint
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::stats::enums::stats_event::StatsEvent;
//!
//! // Update statistics
//! tracker.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1);
//!
//! // Read statistics
//! let stats = tracker.stats.get_stats();
//! ```

/// Statistics event enumeration.
pub mod enums;

/// Implementation blocks for statistics operations.
pub mod impls;

/// Statistics data structures (atomic counters).
pub mod structs;

/// Unit tests for statistics functionality.
pub mod tests;