use serde::{Deserialize, Serialize};

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    pub uuid: String,
    pub key: InfoHash,
    pub uploaded: i64,
    pub downloaded: i64,
    pub completed: i64,
    pub updated: i64,
    pub active: i64,
}

impl TorrentTracker {
    pub async fn check_user_key(&self, hash: InfoHash) -> bool
    {
        let users_arc = self.users.clone();

        if users_arc.get(&hash).is_some() { return true; }

        false
    }
}