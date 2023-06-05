use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Add;

use crate::common::UserId;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    pub uuid: String,
    pub key: UserId,
    pub uploaded: i64,
    pub downloaded: i64,
    pub completed: i64,
    pub updated: i64,
    pub active: i64,
}

impl TorrentTracker {
    pub async fn get_user(&self, user_key: UserId) -> Option<UserEntryItem>
    {
        let users_arc = self.users.clone();
        let user = users_arc.get(&user_key).map(|data| data.value().clone());

        user
    }

    pub async fn get_users_chunk(&self, skip: u64, amount: u64) -> HashMap<UserId, UserEntryItem>
    {
        let users_arc = self.users.clone();

        let mut users_return: HashMap<UserId, UserEntryItem> = HashMap::new();
        let mut current_count: u64 = 0;
        let mut handled_count: u64 = 0;
        for item in users_arc.iter() {
            if current_count < skip {
                current_count = current_count.add(1);
                continue;
            }
            if handled_count >= amount { break; }
            users_return.insert(*item.key(), item.value().clone());
            current_count = current_count.add(1);
            handled_count = handled_count.add(1);
        }

        users_return
    }

    pub async fn add_user(&self, user_key: UserId, user_entry_item: UserEntryItem)
    {
        let users_arc = self.users.clone();

        users_arc.insert(user_key, user_entry_item);

        self.set_stats(StatsEvent::Users, users_arc.len() as i64).await;
    }

    pub async fn remove_user(&self, user_key: UserId)
    {
        let users_arc = self.users.clone();

        users_arc.remove(&user_key);

        self.set_stats(StatsEvent::Users, users_arc.len() as i64).await;
    }

    pub async fn check_user_key(&self, hash: UserId) -> bool
    {
        let users_arc = self.users.clone();

        if users_arc.get(&hash).is_some() { return true; }

        false
    }
}