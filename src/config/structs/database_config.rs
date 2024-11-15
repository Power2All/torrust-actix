use serde::{Deserialize, Serialize};
use crate::database::enums::database_drivers::DatabaseDrivers;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    pub engine: DatabaseDrivers,
    pub path: String,
    pub persistent: bool,
    pub persistent_interval: u64,
    pub insert_vacant: bool,
    pub remove_action: bool,
    pub update_completed: bool,
    pub update_peers: bool
}