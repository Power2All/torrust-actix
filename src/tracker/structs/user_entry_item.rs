use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::user_id::UserId;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    pub uuid: String,
    pub key: UserId,
    pub uploaded: u64,
    pub downloaded: u64,
    pub completed: u64,
    pub updated: u64,
    pub active: u8,
    #[serde(skip_serializing, skip_deserializing)]
    pub torrents_active: HashMap<InfoHash, std::time::Instant>
}
