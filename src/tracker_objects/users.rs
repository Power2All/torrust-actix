use log::info;
use async_std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Add;
use std::time::Duration;

use crate::common::{InfoHash, UserId};
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

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

impl TorrentTracker {
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(users) = self.sqlx.load_users(tracker.clone()).await {
            info!("Loaded {} users.", users);
        }
    }

    pub async fn get_user(&self, user_key: UserId) -> Option<UserEntryItem>
    {
        let users_arc = self.users.clone();
        let user = users_arc.get(&user_key).map(|data| data.value().clone());

        user
    }

    pub async fn get_users(&self, users: Vec<UserId>) -> HashMap<UserId, Option<UserEntryItem>>
    {
        let mut return_users = HashMap::new();

        let users_arc = self.users.clone();
        for user_id in users.iter() {
            return_users.insert(*user_id, users_arc.get(user_id).map(|data| data.value().clone()));
        }

        return_users
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

    pub async fn add_users(&self, users: HashMap<UserId, UserEntryItem>, _persistent: bool)
    {
        let users_arc = self.users.clone();

        for (user_id, user_entry_item) in users.iter() {
            users_arc.insert(*user_id, user_entry_item.clone());
        }

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

    pub async fn clean_users_active_torrents(&self, peer_timeout: Duration)
    {
        // Cleaning up active torrents in chunks, to prevent slow behavior.
        let users_arc = self.users.clone();

        let mut start: usize = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        let mut removed_active_torrents = 0u64;

        loop {
            info!("[USERS] Scanning active torrents in users {} to {}", start, (start + size));

            let mut user_index = vec![];
            for item in users_arc.iter().skip(start) {
                user_index.push(*item.key());
                if user_index.len() == size { break; }
            }

            let users = self.get_users(user_index.clone()).await;
            for (user_id, user_entry_item) in users.iter() {
                if user_entry_item.is_some() {
                    let mut user = user_entry_item.clone().unwrap().clone();
                    let mut torrents_active = user.torrents_active.clone();
                    for (info_hash, timestamp) in user.torrents_active.iter() {
                        if timestamp.elapsed() > peer_timeout {
                            torrents_active.remove(info_hash);
                            removed_active_torrents += 1;
                        }
                    }
                    user.torrents_active = torrents_active;
                    self.add_user(*user_id, user).await;
                } else { continue; }
            }

            if user_index.len() != size {
                break;
            }

            start += size;
        }
        info!("[USERS] Removed {} active torrents in users", removed_active_torrents);
    }
}