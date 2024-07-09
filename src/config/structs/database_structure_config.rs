use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseStructureConfig {
    pub db_torrents: String,

    pub table_torrents_info_hash: String,
    pub table_torrents_completed: String,

    pub db_whitelist: String,
    pub table_whitelist_info_hash: String,

    pub db_blacklist: String,
    pub table_blacklist_info_hash: String,

    pub db_keys: String,
    pub table_keys_hash: String,
    pub table_keys_timeout: String,

    pub db_users: String,
    pub table_users_uuid: String,
    pub table_users_key: String,
    pub table_users_uploaded: String,
    pub table_users_downloaded: String,
    pub table_users_completed: String,
    pub table_users_updated: String,
    pub table_users_active: String,
}
