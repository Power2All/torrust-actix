//! Multi-database backend module.
//!
//! This module provides a unified interface for multiple database backends:
//! - SQLite (for lightweight deployments)
//! - MySQL/MariaDB (for production deployments)
//! - PostgreSQL (for production deployments)
//!
//! # Architecture
//!
//! The database layer uses a trait-based design:
//! - `DatabaseBackend` trait defines the interface
//! - Each database has its own connector implementation
//! - `DatabaseConnector` provides unified access
//!
//! # Features
//!
//! - Customizable table and column names
//! - Connection pooling via SQLx
//! - Automatic table creation
//! - Batch operations for efficiency
//! - Query builder for dynamic queries
//!
//! # Supported Operations
//!
//! - Torrent CRUD (insert, update, delete, select)
//! - Whitelist management
//! - Blacklist management
//! - API key management
//! - User account management
//!
//! # Example
//!
//! ```rust,ignore
//! use torrust_actix::database::structs::database_connector::DatabaseConnector;
//!
//! let connector = DatabaseConnector::new(config, true).await;
//! let torrents = connector.load_torrents().await;
//! ```

/// Database driver enumeration (sqlite3, mysql, pgsql).
pub mod enums;

/// Helper functions for SQL query generation.
pub mod helpers;

/// Implementation blocks for database connectors.
pub mod impls;

/// Dynamic SQL query builder.
pub mod query_builder;

/// Data structures for database connections.
pub mod structs;

/// Database backend trait definitions.
pub mod traits;