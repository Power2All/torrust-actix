use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfigKeys {
    pub table_name: String,
    pub column_hash: String,
    pub bin_type_hash: bool,
    pub column_timeout: String
}