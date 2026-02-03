//! Caching layer module supporting Redis and Memcache.
//!
//! This module provides an optional caching layer for peer data to reduce
//! database load and improve response times in high-traffic deployments.
//!
//! # Supported Backends
//!
//! - **Redis**: Recommended for production, supports clustering
//! - **Memcache**: Alternative option, simpler deployment
//!
//! # Architecture
//!
//! The cache layer uses a trait-based design:
//! - `CacheBackend` trait defines the interface
//! - Each backend has its own connector implementation
//! - `CacheConnector` provides unified access
//!
//! # Features
//!
//! - Connection pooling and management
//! - Configurable TTL (Time-To-Live)
//! - Key prefix customization
//! - Automatic reconnection handling
//! - Error handling with graceful degradation
//!
//! # Usage
//!
//! The cache is optional and can be disabled in configuration. When enabled,
//! peer data is cached to reduce database queries during announce requests.
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::cache::structs::cache_connector::CacheConnector;
//!
//! let connector = CacheConnector::new(config).await?;
//! // Cache operations...
//! ```

/// Cache engine enumeration (redis, memcache).
pub mod enums;

/// Error types for cache operations.
pub mod errors;

/// Implementation blocks for cache connectors.
pub mod impls;

/// Data structures for cache connections.
pub mod structs;

/// Cache backend trait definitions.
pub mod traits;