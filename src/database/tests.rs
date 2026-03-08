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
                "ON CONFLICT (info_hash) DO UPDATE SET seeds=excluded.seeds, peers=excluded.peers"
            );
        }

        #[test]
        fn test_limit_offset() {
            assert_eq!(database::limit_offset(DatabaseDrivers::sqlite3, 0, 100), "LIMIT 0, 100");
            assert_eq!(database::limit_offset(DatabaseDrivers::mysql, 100, 50), "LIMIT 100, 50");
            assert_eq!(database::limit_offset(DatabaseDrivers::pgsql, 100, 50), "LIMIT 50 OFFSET 100");
        }
    }
}