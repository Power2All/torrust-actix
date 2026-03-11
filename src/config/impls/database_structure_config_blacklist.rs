use crate::config::structs::database_structure_config_blacklist::DatabaseStructureConfigBlacklist;

impl Default for DatabaseStructureConfigBlacklist {
    fn default() -> Self {
        DatabaseStructureConfigBlacklist {
            table_name: String::from("blacklist"),
            column_infohash: String::from("infohash"),
            bin_type_infohash: true,
        }
    }
}