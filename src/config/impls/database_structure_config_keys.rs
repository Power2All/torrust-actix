use crate::config::structs::database_structure_config_keys::DatabaseStructureConfigKeys;

impl Default for DatabaseStructureConfigKeys {
    fn default() -> Self {
        DatabaseStructureConfigKeys {
            table_name: String::from("keys"),
            column_hash: String::from("hash"),
            bin_type_hash: true,
            column_timeout: String::from("timeout"),
        }
    }
}