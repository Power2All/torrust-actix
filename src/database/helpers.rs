use crate::database::enums::database_drivers::DatabaseDrivers;

pub fn format_hash_value(engine: DatabaseDrivers, hex_value: &str, is_binary: bool) -> String {
    if is_binary {
        match engine {
            DatabaseDrivers::sqlite3 => format!("X'{}'", hex_value),
            DatabaseDrivers::mysql => format!("UNHEX('{}')", hex_value),
            DatabaseDrivers::pgsql => format!("decode('{}', 'hex')", hex_value),
        }
    } else {
        format!("'{}'", hex_value)
    }
}

pub fn format_hex_select(engine: DatabaseDrivers, column: &str, is_binary: bool) -> String {
    if is_binary {
        match engine {
            DatabaseDrivers::sqlite3 => {
                format!("hex(`{}`) AS `{}`", column, column)
            }
            DatabaseDrivers::mysql => {
                format!("HEX(`{}`) AS `{}`", column, column)
            }
            DatabaseDrivers::pgsql => {
                format!("encode({}, 'hex') AS {}", column, column)
            }
        }
    } else {
        quote_identifier(engine, column)
    }
}

pub fn quote_identifier(engine: DatabaseDrivers, identifier: &str) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("`{}`", identifier),
        DatabaseDrivers::pgsql => identifier.to_string(),
    }
}

pub fn insert_ignore_prefix(engine: DatabaseDrivers) -> &'static str {
    match engine {
        DatabaseDrivers::sqlite3 => "INSERT OR IGNORE INTO",
        DatabaseDrivers::mysql => "INSERT IGNORE INTO",
        DatabaseDrivers::pgsql => "INSERT INTO",
    }
}

pub fn insert_ignore_suffix(engine: DatabaseDrivers, conflict_column: &str) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => String::new(),
        DatabaseDrivers::pgsql => format!(" ON CONFLICT ({}) DO NOTHING", conflict_column),
    }
}

pub fn update_ignore_prefix(engine: DatabaseDrivers) -> &'static str {
    match engine {
        DatabaseDrivers::sqlite3 => "UPDATE OR IGNORE",
        DatabaseDrivers::mysql => "UPDATE IGNORE",
        DatabaseDrivers::pgsql => "UPDATE",
    }
}

pub fn upsert_conflict_clause(engine: DatabaseDrivers, conflict_column: &str, update_columns: &[&str]) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::pgsql => {
            let updates: Vec<String> = update_columns
                .iter()
                .map(|col| {
                    let quoted = quote_identifier(engine, col);
                    format!("{}=excluded.{}", quoted, quoted)
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
                    format!("{}=VALUES({})", quoted, quoted)
                })
                .collect();
            format!("ON DUPLICATE KEY UPDATE {}", updates.join(", "))
        }
    }
}

pub fn limit_offset(engine: DatabaseDrivers, start: u64, length: u64) -> String {
    match engine {
        DatabaseDrivers::sqlite3 | DatabaseDrivers::mysql => format!("LIMIT {}, {}", start, length),
        DatabaseDrivers::pgsql => format!("LIMIT {} OFFSET {}", length, start),
    }
}

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
        "DELETE FROM {} WHERE {}={}",
        quoted_table,
        quoted_column,
        value
    )
}

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
        "{} {} ({}) VALUES ({}){}",
        prefix,
        quoted_table,
        quoted_column,
        value,
        suffix
    )
}

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
        "SELECT {} FROM {} {}",
        columns,
        quoted_table,
        limit
    )
}

pub fn build_upsert_torrent_query(
    engine: DatabaseDrivers,
    table_name: &str,
    column_infohash: &str,
    value_columns: &[(&str, &str)], // (column_name, value)
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

pub fn build_update_ignore_torrent_query(
    engine: DatabaseDrivers,
    table_name: &str,
    column_infohash: &str,
    set_columns: &[(&str, &str)], // (column_name, value)
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

pub fn engine_name(engine: DatabaseDrivers) -> &'static str {
    match engine {
        DatabaseDrivers::sqlite3 => "SQLite",
        DatabaseDrivers::mysql => "MySQL",
        DatabaseDrivers::pgsql => "PgSQL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hash_value_binary() {
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            format_hash_value(DatabaseDrivers::sqlite3, hash, true),
            "X'0123456789abcdef0123456789abcdef01234567'"
        );
        assert_eq!(
            format_hash_value(DatabaseDrivers::mysql, hash, true),
            "UNHEX('0123456789abcdef0123456789abcdef01234567')"
        );
        assert_eq!(
            format_hash_value(DatabaseDrivers::pgsql, hash, true),
            "decode('0123456789abcdef0123456789abcdef01234567', 'hex')"
        );
    }

    #[test]
    fn test_format_hash_value_text() {
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            format_hash_value(DatabaseDrivers::sqlite3, hash, false),
            "'0123456789abcdef0123456789abcdef01234567'"
        );
        assert_eq!(
            format_hash_value(DatabaseDrivers::mysql, hash, false),
            "'0123456789abcdef0123456789abcdef01234567'"
        );
        assert_eq!(
            format_hash_value(DatabaseDrivers::pgsql, hash, false),
            "'0123456789abcdef0123456789abcdef01234567'"
        );
    }

    #[test]
    fn test_upsert_conflict_clause() {
        let columns = &["seeds", "peers"];
        assert_eq!(
            upsert_conflict_clause(DatabaseDrivers::sqlite3, "info_hash", columns),
            "ON CONFLICT (`info_hash`) DO UPDATE SET `seeds`=excluded.`seeds`, `peers`=excluded.`peers`"
        );
        assert_eq!(
            upsert_conflict_clause(DatabaseDrivers::mysql, "info_hash", columns),
            "ON DUPLICATE KEY UPDATE `seeds`=VALUES(`seeds`), `peers`=VALUES(`peers`)"
        );
        assert_eq!(
            upsert_conflict_clause(DatabaseDrivers::pgsql, "info_hash", columns),
            "ON CONFLICT (info_hash) DO UPDATE SET seeds=excluded.seeds, peers=excluded.peers"
        );
    }

    #[test]
    fn test_limit_offset() {
        assert_eq!(limit_offset(DatabaseDrivers::sqlite3, 0, 100), "LIMIT 0, 100");
        assert_eq!(limit_offset(DatabaseDrivers::mysql, 100, 50), "LIMIT 100, 50");
        assert_eq!(limit_offset(DatabaseDrivers::pgsql, 100, 50), "LIMIT 50 OFFSET 100");
    }
}