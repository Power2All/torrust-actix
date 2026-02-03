//! Database connector structures.

/// Main database connector providing unified interface.
pub mod database_connector;

/// SQLite-specific database connector implementation.
pub mod database_connector_sqlite;

/// MySQL/MariaDB-specific database connector implementation.
pub mod database_connector_mysql;

/// PostgreSQL-specific database connector implementation.
pub mod database_connector_pgsql;