use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfigUsers {
    pub database_name: String,
    pub id_uuid: bool,
    pub column_uuid: String,
    pub column_id: String,
    pub column_key: String,
    pub bin_type_key: bool,
    pub column_uploaded: String,
    pub column_downloaded: String,
    pub column_completed: String,
    pub column_updated: String,
    pub column_active: String
}
