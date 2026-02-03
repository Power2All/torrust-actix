//! SSL/TLS certificate management module.
//!
//! This module provides certificate storage, hot-reloading, and dynamic
//! resolution for HTTPS endpoints. It supports multiple server identities
//! and SNI-based certificate selection.
//!
//! # Features
//!
//! - Hot-reload certificates without server restart
//! - Multiple certificate stores per server type
//! - SNI (Server Name Indication) support
//! - Certificate validation and error handling
//! - Thread-safe certificate updates
//!
//! # Certificate Storage
//!
//! Certificates are organized by server identifier:
//! - API servers: `ApiServer(<address>)`
//! - HTTP servers: `HttpServer(<address>)`
//! - WebSocket servers: `WsMaster(<address>)` / `WsSlave`
//!
//! # Hot Reload
//!
//! Certificates can be reloaded at runtime via the API endpoint:
//! `POST /api/certificate/reload`
//!
//! This allows updating SSL certificates (e.g., from Let's Encrypt)
//! without restarting the tracker.
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::ssl::certificate_store::{CertificateStore, ServerIdentifier};
//!
//! let store = CertificateStore::new();
//! let server_id = ServerIdentifier::HttpServer("127.0.0.1:443".to_string());
//!
//! store.load_certificate(server_id, "cert.pem", "key.pem")?;
//! ```

/// Certificate storage with hot-reload support.
pub mod certificate_store;

/// Dynamic certificate resolver for SNI-based selection.
pub mod certificate_resolver;