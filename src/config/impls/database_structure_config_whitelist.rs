use crate::config::structs::database_structure_config_whitelist::DatabaseStructureConfigWhitelist;

impl Default for DatabaseStructureConfigWhitelist {
    fn default() -> Self {
        DatabaseStructureConfigWhitelist {
            table_name: String::from("whitelist"),
            column_infohash: String::from("infohash"),
            bin_type_infohash: true,
        }
    }
}