use crate::database::enums::database_drivers::DatabaseDrivers;

/// Formats a hex hash as an engine-specific SQL literal: a binary literal
/// (`X'..'` / `UNHEX` / `decode`) when `is_binary` is set, otherwise a quoted string.
///
/// Safe against injection because callers only pass hex-encoded values.
pub fn format_hash_value(engine: DatabaseDrivers, hex_value: &str, is_binary: bool) -> String {
    if is_binary {
        match engine {
            DatabaseDrivers::sqlite3 => format!("X'{hex_value}'"),
            DatabaseDrivers::mysql => format!("UNHEX('{hex_value}')"),
            DatabaseDrivers::pgsql => format!("decode('{hex_value}', 'hex')"),
        }
    } else {
        format!("'{hex_value}'")
    }
}

/// Builds the SELECT expression that returns a hash column as hex text,
/// wrapping binary columns in the engine's `HEX()`/`hex()` function.
pub fn format_hex_select(engine: DatabaseDrivers, column: &str, is_binary: bool) -> String {
    if is_binary {
        match engine {
            DatabaseDrivers::sqlite3 => {
                format!("hex(`{column}`) AS `{column}`")
            }
            DatabaseDrivers::mysql => {
                format!("HEX(`{column}`) AS `{column}`")
            }
            DatabaseDrivers::pgsql => {
                quote_identifier(engine, column)
            }
        }
    } else {
        quote_identifier(engine, column)
    }
}

/// Quotes a table or column identifier for the given engine (backticks for
/// SQLite/MySQL, double quotes for PostgreSQL).
pub fn quote_identifier(engine: DatabaseDrivers, identifier: &str) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("`{identifier}`"),
        DatabaseDrivers::pgsql => format!("\"{}\"", identifier.replace('"', "\"\"")),
    }
}

/// Returns the engine's "insert, ignore duplicates" statement prefix.
pub fn insert_ignore_prefix(engine: DatabaseDrivers) -> &'static str {
    match engine {
        DatabaseDrivers::sqlite3 => "INSERT OR IGNORE INTO",
        DatabaseDrivers::mysql => "INSERT IGNORE INTO",
        DatabaseDrivers::pgsql => "INSERT INTO",
    }
}

/// Returns the engine's "insert, ignore duplicates" statement suffix
/// (`ON CONFLICT .. DO NOTHING` for PostgreSQL, empty otherwise).
pub fn insert_ignore_suffix(engine: DatabaseDrivers, conflict_column: &str) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => String::new(),
        DatabaseDrivers::pgsql => format!(" ON CONFLICT ({conflict_column}) DO NOTHING"),
    }
}

/// Returns the engine's "update, ignore errors" statement prefix.
pub fn update_ignore_prefix(engine: DatabaseDrivers) -> &'static str {
    match engine {
        DatabaseDrivers::sqlite3 => "UPDATE OR IGNORE",
        DatabaseDrivers::mysql => "UPDATE IGNORE",
        DatabaseDrivers::pgsql => "UPDATE",
    }
}

/// Builds the engine-specific upsert conflict clause (`ON CONFLICT .. DO UPDATE` or
/// `ON DUPLICATE KEY UPDATE`) updating the given columns.
pub fn upsert_conflict_clause(engine: DatabaseDrivers, conflict_column: &str, update_columns: &[&str]) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::pgsql => {
            let updates: Vec<String> = update_columns
                .iter()
                .map(|col| {
                    let quoted = quote_identifier(engine, col);
                    format!("{quoted}=excluded.{quoted}")
                })
                .collect();
            format!(
                "ON CONFLICT ({}) DO UPDATE SET {}",
                quote_identifier(engine, conflict_column),
                updates.join(", ")
            )
        }
        DatabaseDrivers::mysql => {
            let updates: Vec<String> = update_columns
                .iter()
                .map(|col| {
                    let quoted = quote_identifier(engine, col);
                    format!("{quoted}=VALUES({quoted})")
                })
                .collect();
            format!("ON DUPLICATE KEY UPDATE {}", updates.join(", "))
        }
    }
}

/// Builds the engine-specific `LIMIT`/`OFFSET` clause for paged loading.
pub fn limit_offset(engine: DatabaseDrivers, start: u64, length: u64) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("LIMIT {start}, {length}"),
        DatabaseDrivers::pgsql => format!("LIMIT {length} OFFSET {start}"),
    }
}

/// Builds a `DELETE .. WHERE hash_column = value` statement for the given engine.
pub fn build_delete_hash_query(
    engine: DatabaseDrivers,
    table_name: &str,
    column_name: &str,
    hash_value: &str,
    is_binary: bool,
) -> String {
    let quoted_table = quote_identifier(engine, table_name);
    let quoted_column = quote_identifier(engine, column_name);
    let value = format_hash_value(engine, hash_value, is_binary);
    format!(
        "DELETE FROM {quoted_table} WHERE {quoted_column}={value}"
    )
}

/// Builds an insert-if-absent statement for a single hash value.
pub fn build_insert_ignore_hash_query(
    engine: DatabaseDrivers,
    table_name: &str,
    column_name: &str,
    hash_value: &str,
    is_binary: bool,
) -> String {
    let quoted_table = quote_identifier(engine, table_name);
    let quoted_column = quote_identifier(engine, column_name);
    let value = format_hash_value(engine, hash_value, is_binary);
    let prefix = insert_ignore_prefix(engine);
    let suffix = insert_ignore_suffix(engine, column_name);
    format!(
        "{prefix} {quoted_table} ({quoted_column}) VALUES ({value}){suffix}"
    )
}

/// Builds a paged `SELECT` returning the hash column as hex plus any additional columns.
pub fn build_select_hash_query(
    engine: DatabaseDrivers,
    table_name: &str,
    hash_column: &str,
    additional_columns: &[&str],
    is_binary: bool,
    start: u64,
    length: u64,
) -> String {
    let quoted_table = quote_identifier(engine, table_name);
    let hash_select = format_hex_select(engine, hash_column, is_binary);
    let columns = if additional_columns.is_empty() {
        hash_select
    } else {
        let additional: Vec<String> = additional_columns
            .iter()
            .map(|col| quote_identifier(engine, col))
            .collect();
        format!("{}, {}", hash_select, additional.join(", "))
    };
    let limit = limit_offset(engine, start, length);
    format!(
        "SELECT {columns} FROM {quoted_table} {limit}"
    )
}

/// Builds an upsert statement for a torrent row keyed by info-hash, inserting
/// `value_columns` and updating `update_columns` on conflict.
pub fn build_upsert_torrent_query(
    engine: DatabaseDrivers,
    table_name: &str,
    column_infohash: &str,
    value_columns: &[(&str, &str)],
    update_columns: &[&str],
    hash_value: &str,
    is_binary: bool,
) -> String {
    let quoted_table = quote_identifier(engine, table_name);
    let quoted_infohash = quote_identifier(engine, column_infohash);
    let hash_val = format_hash_value(engine, hash_value, is_binary);
    let mut col_names = vec![quoted_infohash.clone()];
    let mut col_values = vec![hash_val];
    for (col, val) in value_columns {
        col_names.push(quote_identifier(engine, col));
        col_values.push(val.to_string());
    }
    let conflict = upsert_conflict_clause(engine, column_infohash, update_columns);
    format!(
        "INSERT INTO {} ({}) VALUES ({}) {}",
        quoted_table,
        col_names.join(", "),
        col_values.join(", "),
        conflict
    )
}

/// Builds an `UPDATE .. WHERE infohash = value` statement that ignores conflicts.
pub fn build_update_ignore_torrent_query(
    engine: DatabaseDrivers,
    table_name: &str,
    column_infohash: &str,
    set_columns: &[(&str, &str)],
    hash_value: &str,
    is_binary: bool,
) -> String {
    let quoted_table = quote_identifier(engine, table_name);
    let quoted_infohash = quote_identifier(engine, column_infohash);
    let hash_val = format_hash_value(engine, hash_value, is_binary);
    let prefix = update_ignore_prefix(engine);
    let sets: Vec<String> = set_columns
        .iter()
        .map(|(col, val)| format!("{}={}", quote_identifier(engine, col), val))
        .collect();
    format!(
        "{} {} SET {} WHERE {}={}",
        prefix,
        quoted_table,
        sets.join(", "),
        quoted_infohash,
        hash_val
    )
}

/// Returns the human-readable engine name used in log prefixes.
pub fn engine_name(engine: DatabaseDrivers) -> &'static str {
    match engine {
        DatabaseDrivers::sqlite3 => "SQLite",
        DatabaseDrivers::mysql => "MySQL",
        DatabaseDrivers::pgsql => "PgSQL",
    }
}