use crate::config::structs::database_structure_config_torrents::DatabaseStructureConfigTorrents;

impl Default for DatabaseStructureConfigTorrents {
    fn default() -> Self {
        DatabaseStructureConfigTorrents {
            table_name: String::from("torrents"),
            column_infohash: String::from("infohash"),
            bin_type_infohash: true,
            column_seeds: String::from("seeds"),
            column_peers: String::from("peers"),
            column_completed: String::from("completed"),
            persistent: None,
        }
    }
}