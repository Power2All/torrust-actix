use crate::config::structs::database_structure_config_users::DatabaseStructureConfigUsers;

impl Default for DatabaseStructureConfigUsers {
    fn default() -> Self {
        DatabaseStructureConfigUsers {
            table_name: String::from("users"),
            id_uuid: true,
            column_uuid: String::from("uuid"),
            column_id: String::from("id"),
            column_key: String::from("key"),
            bin_type_key: true,
            column_uploaded: String::from("uploaded"),
            column_downloaded: String::from("downloaded"),
            column_completed: String::from("completed"),
            column_updated: String::from("updated"),
            column_active: String::from("active"),
            persistent: None,
        }
    }
}