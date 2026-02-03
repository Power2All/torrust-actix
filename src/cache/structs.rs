//! Cache connector structures.

/// Main cache connector providing unified interface.
pub mod cache_connector;

/// Redis-specific cache connector implementation.
pub mod cache_connector_redis;

/// Memcache-specific cache connector implementation.
pub mod cache_connector_memcache;