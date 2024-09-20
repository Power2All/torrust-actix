use serde::{Deserialize, Serialize};
use crate::config::structs::database_structure_config_blacklist::DatabaseStructureConfigBlacklist;
use crate::config::structs::database_structure_config_keys::DatabaseStructureConfigKeys;
use crate::config::structs::database_structure_config_torrents::DatabaseStructureConfigTorrents;
use crate::config::structs::database_structure_config_users::DatabaseStructureConfigUsers;
use crate::config::structs::database_structure_config_whitelist::DatabaseStructureConfigWhitelist;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfig {
    pub torrents: Option<DatabaseStructureConfigTorrents>,
    pub whitelist: Option<DatabaseStructureConfigWhitelist>,
    pub blacklist: Option<DatabaseStructureConfigBlacklist>,
    pub keys: Option<DatabaseStructureConfigKeys>,
    pub users: Option<DatabaseStructureConfigUsers>
}
