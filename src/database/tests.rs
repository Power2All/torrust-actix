#[cfg(test)]
mod database_tests {
    use crate::database::database;
    use crate::database::enums::database_drivers::DatabaseDrivers;

    mod helpers_tests {
        use super::*;

        #[test]
        fn test_format_hash_value_binary() {
            let hash = "0123456789abcdef0123456789abcdef01234567";
            assert_eq!(
                database::format_hash_value(DatabaseDrivers::sqlite3, hash, true),
                "X'0123456789abcdef0123456789abcdef01234567'"
            );
            assert_eq!(
                database::format_hash_value(DatabaseDrivers::mysql, hash, true),
                "UNHEX('0123456789abcdef0123456789abcdef01234567')"
            );
            assert_eq!(
                database::format_hash_value(DatabaseDrivers::pgsql, hash, true),
                "decode('0123456789abcdef0123456789abcdef01234567', 'hex')"
            );
        }

        #[test]
        fn test_format_hash_value_text() {
            let hash = "0123456789abcdef0123456789abcdef01234567";
            assert_eq!(
                database::format_hash_value(DatabaseDrivers::sqlite3, hash, false),
                "'0123456789abcdef0123456789abcdef01234567'"
            );
            assert_eq!(
                database::format_hash_value(DatabaseDrivers::mysql, hash, false),
                "'0123456789abcdef0123456789abcdef01234567'"
            );
            assert_eq!(
                database::format_hash_value(DatabaseDrivers::pgsql, hash, false),
                "'0123456789abcdef0123456789abcdef01234567'"
            );
        }

        #[test]
        fn test_upsert_conflict_clause() {
            let columns = &["seeds", "peers"];
            assert_eq!(
                database::upsert_conflict_clause(DatabaseDrivers::sqlite3, "info_hash", columns),
                "ON CONFLICT (`info_hash`) DO UPDATE SET `seeds`=excluded.`seeds`, `peers`=excluded.`peers`"
            );
            assert_eq!(
                database::upsert_conflict_clause(DatabaseDrivers::mysql, "info_hash", columns),
                "ON DUPLICATE KEY UPDATE `seeds`=VALUES(`seeds`), `peers`=VALUES(`peers`)"
            );
            assert_eq!(
                database::upsert_conflict_clause(DatabaseDrivers::pgsql, "info_hash", columns),
                "ON CONFLICT (\"info_hash\") DO UPDATE SET \"seeds\"=excluded.\"seeds\", \"peers\"=excluded.\"peers\""
            );
        }

        #[test]
        fn test_limit_offset() {
            assert_eq!(database::limit_offset(DatabaseDrivers::sqlite3, 0, 100), "LIMIT 0, 100");
            assert_eq!(database::limit_offset(DatabaseDrivers::mysql, 100, 50), "LIMIT 100, 50");
            assert_eq!(database::limit_offset(DatabaseDrivers::pgsql, 100, 50), "LIMIT 50 OFFSET 100");
        }

        #[test]
        fn test_quote_identifier_escapes_its_own_quote_character() {
            assert_eq!(database::quote_identifier(DatabaseDrivers::sqlite3, "a`b"), "`a``b`");
            assert_eq!(database::quote_identifier(DatabaseDrivers::mysql, "a`b"), "`a``b`");
            assert_eq!(database::quote_identifier(DatabaseDrivers::pgsql, "a\"b"), "\"a\"\"b\"");
        }
    }

    mod query_builder_tests {
        use super::*;
        use crate::database::structs::query_builder::QueryBuilder;

        #[test]
        fn test_text_literal_escaping_per_engine() {
            let value = r"O'Brien\x";

            // SQLite has no backslash escapes: only the quote is doubled.
            assert_eq!(QueryBuilder::new(DatabaseDrivers::sqlite3).text_literal(value), r"'O''Brien\x'");

            // MySQL treats backslash as an escape by default, so it is doubled too.
            assert_eq!(QueryBuilder::new(DatabaseDrivers::mysql).text_literal(value), r"'O''Brien\\x'");

            // PostgreSQL uses an E'' literal so the result is correct regardless of the
            // server's `standard_conforming_strings` setting.
            assert_eq!(QueryBuilder::new(DatabaseDrivers::pgsql).text_literal(value), r"E'O''Brien\\x'");
        }

        #[test]
        fn test_text_literal_cannot_terminate_the_literal_early() {
            // A trailing backslash is the case an ordinary PostgreSQL literal gets wrong when
            // `standard_conforming_strings` is off: the backslash would escape the closing quote.
            for engine in [DatabaseDrivers::sqlite3, DatabaseDrivers::mysql, DatabaseDrivers::pgsql] {
                let literal = QueryBuilder::new(engine).text_literal(r"trailing\");
                assert!(literal.ends_with('\''), "{engine:?} literal must be closed: {literal}");
                if engine != DatabaseDrivers::sqlite3 {
                    assert!(literal.ends_with(r"\\'"), "{engine:?} must escape the trailing backslash: {literal}");
                }
            }
        }
    }
}