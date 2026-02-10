use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfigBlacklist {
    pub table_name: String,
    pub column_infohash: String,
    pub bin_type_infohash: bool
}