use crate::database::enums::database_drivers::DatabaseDrivers;

#[derive(Debug, Clone)]
pub struct QueryBuilder {
    pub engine: DatabaseDrivers,
}

impl QueryBuilder {
    pub fn new(engine: DatabaseDrivers) -> Self {
        Self { engine }
    }

    pub fn quote_identifier(&self, identifier: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("`{}`", identifier),
            DatabaseDrivers::pgsql => identifier.to_string(),
        }
    }

    pub fn binary_literal(&self, hex_value: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 => format!("X'{}'", hex_value),
            DatabaseDrivers::mysql => format!("X'{}'", hex_value),
            DatabaseDrivers::pgsql => format!("'\\x{}'::bytea", hex_value),
        }
    }

    pub fn text_literal(&self, value: &str) -> String {
        format!("'{}'", value)
    }

    pub fn upsert_conflict_clause(&self, conflict_column: &str, update_columns: &[&str]) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::pgsql => {
                let updates: Vec<String> = update_columns
                    .iter()
                    .map(|col| {
                        let quoted = self.quote_identifier(col);
                        format!("{}=excluded.{}", quoted, quoted)
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
                        format!("{}=VALUES({})", quoted, quoted)
                    })
                    .collect();
                format!("ON DUPLICATE KEY UPDATE {}", updates.join(", "))
            }
        }
    }

    pub fn insert_ignore_prefix(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "INSERT OR IGNORE INTO",
            DatabaseDrivers::mysql => "INSERT IGNORE INTO",
            DatabaseDrivers::pgsql => "INSERT INTO",
        }
    }

    pub fn insert_ignore_suffix(&self, conflict_column: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => String::new(),
            DatabaseDrivers::pgsql => format!(" ON CONFLICT ({}) DO NOTHING", self.quote_identifier(conflict_column)),
        }
    }

    pub fn update_ignore_prefix(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "UPDATE OR IGNORE",
            DatabaseDrivers::mysql => "UPDATE IGNORE",
            DatabaseDrivers::pgsql => "UPDATE",
        }
    }

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

    pub fn unhex(&self, hex_value: &str) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 => format!("X'{}'", hex_value),
            DatabaseDrivers::mysql => format!("UNHEX('{}')", hex_value),
            DatabaseDrivers::pgsql => format!("decode('{}', 'hex')", hex_value),
        }
    }

    pub fn limit_offset(&self, offset: u64, limit: u64) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("LIMIT {}, {}", offset, limit),
            DatabaseDrivers::pgsql => format!("LIMIT {} OFFSET {}", limit, offset),
        }
    }

    pub fn auto_increment(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "AUTOINCREMENT",
            DatabaseDrivers::mysql => "AUTO_INCREMENT",
            DatabaseDrivers::pgsql => "",
        }
    }

    pub fn integer_type(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "INTEGER",
            DatabaseDrivers::mysql => "INT",
            DatabaseDrivers::pgsql => "integer",
        }
    }

    pub fn bigint_type(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "INTEGER",
            DatabaseDrivers::mysql => "BIGINT UNSIGNED",
            DatabaseDrivers::pgsql => "bigint",
        }
    }

    pub fn binary_type(&self, size: usize) -> String {
        match self.engine {
            DatabaseDrivers::sqlite3 => "BLOB".to_string(),
            DatabaseDrivers::mysql => format!("BINARY({})", size),
            DatabaseDrivers::pgsql => "bytea".to_string(),
        }
    }

    pub fn text_type(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "TEXT",
            DatabaseDrivers::mysql => "VARCHAR(40)",
            DatabaseDrivers::pgsql => "character(40)",
        }
    }

    pub fn engine_name(&self) -> &'static str {
        match self.engine {
            DatabaseDrivers::sqlite3 => "SQLite",
            DatabaseDrivers::mysql => "MySQL",
            DatabaseDrivers::pgsql => "PgSQL",
        }
    }
}