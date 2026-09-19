use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::database::structs::database_connector_pgsql::DatabaseConnectorPgSQL;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;

/// A connection pool for whichever database engine is configured.
///
/// An enum rather than one `Option` per engine plus an `Option<DatabaseDrivers>` discriminant:
/// the three `database_connector` constructors are the only way to build one and each fills
/// exactly one engine, so the mismatched combinations that shape allowed — engine set with no
/// pool, two pools at once, no engine at all — were unreachable states that every method still
/// had to answer for with an `Err(Error::RowNotFound)` arm.
#[derive(Debug, Clone)]
pub enum DatabaseConnector {
    MySQL(DatabaseConnectorMySQL),
    SQLite(DatabaseConnectorSQLite),
    PgSQL(DatabaseConnectorPgSQL),
}

impl DatabaseConnector {
    /// The engine this connector talks to, for SQL dialect choices and tracing tags.
    pub(crate) fn engine(&self) -> DatabaseDrivers {
        match self {
            DatabaseConnector::MySQL(_) => DatabaseDrivers::mysql,
            DatabaseConnector::SQLite(_) => DatabaseDrivers::sqlite3,
            DatabaseConnector::PgSQL(_) => DatabaseDrivers::pgsql,
        }
    }
}
