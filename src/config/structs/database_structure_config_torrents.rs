use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfigTorrents {
    pub table_name: String,
    pub column_infohash: String,
    pub bin_type_infohash: bool,
    pub column_seeds: String,
    pub column_peers: String,
    pub column_completed: String
}