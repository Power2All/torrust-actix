use std::collections::HashMap;
use crate::common::UserId;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;
use crate::tracker_objects::users::UserEntryItem;

impl TorrentTracker {
    pub async fn add_users_update(&self, user_id: UserId, user_entry_item: UserEntryItem)
    {
        let users_updates_arc = self.users_updates.clone();

        users_updates_arc.insert(user_id, user_entry_item);
        let users_update_count = users_updates_arc.len() as i64;

        self.set_stats(StatsEvent::UsersUpdates, users_update_count).await;
    }

    pub async fn add_users_updates(&self, updates: HashMap<UserId, UserEntryItem>)
    {
        let users_updates_arc = self.users_updates.clone();

        let mut users_update_count = 0;

        for (user_id, user_entry_item) in updates.iter() {
            users_updates_arc.insert(*user_id, user_entry_item.clone());
            users_update_count = users_updates_arc.len();
        }

        self.set_stats(StatsEvent::UsersUpdates, users_update_count as i64).await;
    }

    pub async fn get_users_update(&self) -> HashMap<UserId, UserEntryItem>
    {
        let users_updates_arc = self.users_updates.clone();

        let mut users_updates = HashMap::new();
        for item in users_updates_arc.iter() { users_updates.insert(*item.key(), item.value().clone()); }

        users_updates
    }

    pub async fn remove_users_update(&self, user_id: UserId)
    {
        let users_updates_arc = self.users_updates.clone();

        users_updates_arc.remove(&user_id);
        let users_update_count = users_updates_arc.len();

        self.set_stats(StatsEvent::UsersUpdates, users_update_count as i64).await;
    }

    pub async fn remove_users_updates(&self, hashes: Vec<UserId>)
    {
        let users_updates_arc = self.users_updates.clone();

        let mut users_update_count = 0;

        for user_id in hashes.iter() {
            users_updates_arc.remove(user_id);
            users_update_count = users_updates_arc.len();
        }

        self.set_stats(StatsEvent::UsersUpdates, users_update_count as i64).await;
    }

    pub async fn transfer_users_updates_to_users_shadow(&self)
    {
        let users_updates_arc = self.users_updates.clone();

        for item in users_updates_arc.iter() {
            self.add_users_shadow(*item.key(), item.value().clone()).await;
            users_updates_arc.remove(item.key());
        }

        self.set_stats(StatsEvent::UsersUpdates, 0).await;
    }
}