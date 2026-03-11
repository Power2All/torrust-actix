//! # Torrust-Actix
//!
//! A high-performance, feature-rich BitTorrent tracker built with [Actix Web](https://actix.rs/).
//!
//! ## Supported protocols
//!
//! - **HTTP/HTTPS** — standard BitTorrent announce and scrape ([BEP 3], [BEP 23], [BEP 48])
//! - **UDP** — UDP tracker protocol ([BEP 15], [BEP 41])
//! - **WebRTC** — browser-native peer exchange via RtcTorrent (no plugin required)
//!
//! ## Key features
//!
//! - Full IPv4 and IPv6 support ([BEP 7])
//! - SQLite 3, MySQL and PostgreSQL persistence via SQLx
//! - Whitelist, blacklist, torrent keys and per-user tracking
//! - Stand-alone / master / slave cluster mode over WebSockets
//! - Optional Redis or Memcache caching layer
//! - Cloudflare Simple Proxy Protocol support for UDP
//! - Sentry error-tracking integration
//! - Swagger UI built into the API server
//! - Configurable LZ4/Zstd in-memory compression for RtcTorrent SDP data
//!
//! [BEP 3]: https://www.bittorrent.org/beps/bep_0003.html
//! [BEP 7]: https://www.bittorrent.org/beps/bep_0007.html
//! [BEP 15]: https://www.bittorrent.org/beps/bep_0015.html
//! [BEP 23]: https://www.bittorrent.org/beps/bep_0023.html
//! [BEP 41]: https://www.bittorrent.org/beps/bep_0041.html
//! [BEP 48]: https://www.bittorrent.org/beps/bep_0048.html

/// HTTP and WebSocket API server with Swagger UI.
pub mod api;
/// Pluggable caching layer (Redis and Memcache backends).
pub mod cache;
/// Shared utilities, error types, and compression helpers.
pub mod common;
/// TOML-based configuration structures and environment-variable overrides.
pub mod config;
/// Async database abstraction (SQLite 3, MySQL, PostgreSQL).
pub mod database;
/// HTTP/HTTPS BitTorrent tracker protocol implementation.
pub mod http;
/// IP validation, request-size limits, and other security helpers.
pub mod security;
/// TLS certificate management and hot-reload support.
pub mod ssl;
/// Atomic statistics counters for all tracker metrics.
pub mod stats;
/// Command-line argument definitions.
pub mod structs;
/// Core tracker state, peer management, and announce/scrape handling.
pub mod tracker;
/// UDP BitTorrent tracker protocol implementation ([BEP 15]).
pub mod udp;
/// Miscellaneous utility functions.
pub mod utils;
/// WebSocket cluster communication (master ↔ slave).
pub mod websocket;
/// Bridge between the HTTP announce endpoint and the RtcTorrent WebRTC signalling layer.
pub mod rtctorrent_bridge;