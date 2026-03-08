use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::user_id::UserId;
use serde::{
    Deserialize,
    Serialize
};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    pub key: UserId,
    pub user_id: Option<u64>,
    pub user_uuid: Option<String>,
    pub uploaded: u64,
    pub downloaded: u64,
    pub completed: u64,
    pub updated: u64,
    pub active: u8,
    pub torrents_active: BTreeMap<InfoHash, u64>
}