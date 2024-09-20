use serde::{Deserialize, Serialize};
use crate::database::enums::database_drivers::DatabaseDrivers;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    pub engine: Option<DatabaseDrivers>,
    pub path: Option<String>,
    pub persistent: bool,
    pub persistent_interval: Option<u64>,
    pub insert_vacant: bool,
    pub update_completed: bool,
    pub update_peers: bool
}
