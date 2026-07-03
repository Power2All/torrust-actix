use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::query_builder::QueryBuilder;

impl QueryBuilder {
    /// Creates a query builder bound to the given database engine.
    pub fn new(engine: DatabaseDrivers) -> Self {
        Self { engine }
    }

    /// Quotes a table or column identifier for the bound engine.
    pub fn quote_identifier(&self, identifier: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("`{identifier}`"),
            DatabaseDrivers::pgsql => identifier.to_string(),
        }
    }

    /// Formats a hex string as an engine-specific binary literal.
    pub fn binary_literal(&self, hex_value: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 => format!("X'{hex_value}'"),
            DatabaseDrivers::mysql => format!("X'{hex_value}'"),
            DatabaseDrivers::pgsql => format!("'\\x{hex_value}'::bytea"),
        }
    }

    /// Formats a value as a single-quoted SQL string literal.
    pub fn text_literal(&self, value: &str) -> String {
        format!("'{value}'")
    }

    /// Builds the engine-specific upsert conflict clause updating the given columns.
    pub fn upsert_conflict_clause(&self, conflict_column: &str, update_columns: &[&str]) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::pgsql => {
                let updates: Vec<String> = update_columns
                    .iter()
                    .map(|col| {
                        let quoted = self.quote_identifier(col);
                        format!("{quoted}=excluded.{quoted}")
                    })
                    .collect();
                format!(
                    "ON CONFLICT ({}) DO UPDATE SET {}",
                    self.quote_identifier(conflict_column),
                    updates.join(", ")
                )
            }
            DatabaseDrivers::mysql => {
                let updates: Vec<String> = update_columns
                    .iter()
                    .map(|col| {
                        let quoted = self.quote_identifier(col);
                        format!("{quoted}=VALUES({quoted})")
                    })
                    .collect();
                format!("ON DUPLICATE KEY UPDATE {}", updates.join(", "))
            }
        }
    }

    /// Returns the engine's "insert, ignore duplicates" statement prefix.
    pub fn insert_ignore_prefix(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "INSERT OR IGNORE INTO",
            DatabaseDrivers::mysql => "INSERT IGNORE INTO",
            DatabaseDrivers::pgsql => "INSERT INTO",
        }
    }

    /// Returns the engine's "insert, ignore duplicates" statement suffix
    /// (`ON CONFLICT .. DO NOTHING` for PostgreSQL, empty otherwise).
    pub fn insert_ignore_suffix(&self, conflict_column: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => String::new(),
            DatabaseDrivers::pgsql => format!(" ON CONFLICT ({}) DO NOTHING", self.quote_identifier(conflict_column)),
        }
    }

    /// Returns the engine's "update, ignore errors" statement prefix.
    pub fn update_ignore_prefix(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "UPDATE OR IGNORE",
            DatabaseDrivers::mysql => "UPDATE IGNORE",
            DatabaseDrivers::pgsql => "UPDATE",
        }
    }

    /// Builds a SELECT expression returning a binary column as hex text under `alias`.
    pub fn select_hex(&self, column: &str, alias: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => {
                format!("HEX({}) AS {}", self.quote_identifier(column), self.quote_identifier(alias))
            }
            DatabaseDrivers::pgsql => {
                format!("encode({}, 'hex') AS {}", self.quote_identifier(column), alias)
            }
        }
    }

    /// Formats a hex string as the engine's hex-to-binary conversion expression.
    pub fn unhex(&self, hex_value: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 => format!("X'{hex_value}'"),
            DatabaseDrivers::mysql => format!("UNHEX('{hex_value}')"),
            DatabaseDrivers::pgsql => format!("decode('{hex_value}', 'hex')"),
        }
    }

    /// Builds the engine-specific `LIMIT`/`OFFSET` clause.
    pub fn limit_offset(&self, offset: u64, limit: u64) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("LIMIT {offset}, {limit}"),
            DatabaseDrivers::pgsql => format!("LIMIT {limit} OFFSET {offset}"),
        }
    }

    /// Returns the engine's auto-increment column keyword (empty for PostgreSQL).
    pub fn auto_increment(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "AUTOINCREMENT",
            DatabaseDrivers::mysql => "AUTO_INCREMENT",
            DatabaseDrivers::pgsql => "",
        }
    }

    /// Returns the engine's 32-bit integer column type.
    pub fn integer_type(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "INTEGER",
            DatabaseDrivers::mysql => "INT",
            DatabaseDrivers::pgsql => "integer",
        }
    }

    /// Returns the engine's 64-bit integer column type.
    pub fn bigint_type(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "INTEGER",
            DatabaseDrivers::mysql => "BIGINT UNSIGNED",
            DatabaseDrivers::pgsql => "bigint",
        }
    }

    /// Returns the engine's fixed-size binary column type.
    pub fn binary_type(&self, size: usize) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 => "BLOB".to_string(),
            DatabaseDrivers::mysql => format!("BINARY({size})"),
            DatabaseDrivers::pgsql => "bytea".to_string(),
        }
    }

    /// Returns the engine's 40-character text column type (for hex-encoded hashes).
    pub fn text_type(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "TEXT",
            DatabaseDrivers::mysql => "VARCHAR(40)",
            DatabaseDrivers::pgsql => "character(40)",
        }
    }

    /// Returns the human-readable engine name used in log prefixes.
    pub fn engine_name(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "SQLite",
            DatabaseDrivers::mysql => "MySQL",
            DatabaseDrivers::pgsql => "PgSQL",
        }
    }
}