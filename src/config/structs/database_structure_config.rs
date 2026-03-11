use crate::config::structs::database_structure_config_blacklist::DatabaseStructureConfigBlacklist;
use crate::config::structs::database_structure_config_keys::DatabaseStructureConfigKeys;
use crate::config::structs::database_structure_config_torrents::DatabaseStructureConfigTorrents;
use crate::config::structs::database_structure_config_users::DatabaseStructureConfigUsers;
use crate::config::structs::database_structure_config_whitelist::DatabaseStructureConfigWhitelist;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DatabaseStructureConfig {
    #[serde(default)]
    pub torrents: DatabaseStructureConfigTorrents,
    #[serde(default)]
    pub whitelist: DatabaseStructureConfigWhitelist,
    #[serde(default)]
    pub blacklist: DatabaseStructureConfigBlacklist,
    #[serde(default)]
    pub keys: DatabaseStructureConfigKeys,
    #[serde(default)]
    pub users: DatabaseStructureConfigUsers
}