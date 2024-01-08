use std::collections::HashMap;
use async_std::sync::Arc;
use crate::common::UserId;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;
use crate::tracker_objects::users::UserEntryItem;

impl TorrentTracker {
    pub async fn save_users(&self, tracker: Arc<TorrentTracker>) -> bool
    {
        if self.sqlx.save_users(tracker.clone(), self.get_users_shadow().await).await.is_ok() {
            return true;
        }

        false
    }

    pub async fn add_users_shadow(&self, user_id: UserId, user_entry_item: UserEntryItem)
    {
        let users_shadow_arc = self.users_shadow.clone();

        users_shadow_arc.insert(user_id, user_entry_item);
        let users_shadow_count = users_shadow_arc.len();

        self.set_stats(StatsEvent::UsersShadow, users_shadow_count as i64).await;
    }

    pub async fn remove_users_shadow(&self, user_id: UserId)
    {
        let users_shadow_arc = self.users_shadow.clone();

        users_shadow_arc.remove(&user_id);
        let users_shadow_count = users_shadow_arc.len();

        self.set_stats(StatsEvent::UsersShadow, users_shadow_count as i64).await;
    }

    pub async fn remove_users_shadows(&self, hashes: Vec<UserId>)
    {
        let users_shadow_arc = self.users_shadow.clone();

        let mut users_shadow_count = 0;
        for user_id in hashes.iter() {
            users_shadow_arc.remove(user_id);
            users_shadow_count = users_shadow_arc.len();
        }

        self.set_stats(StatsEvent::UsersShadow, users_shadow_count as i64).await;
    }

    pub async fn get_users_shadow(&self) -> HashMap<UserId, UserEntryItem>
    {
        let users_shadow_arc = self.users_shadow.clone();

        let mut users_shadow = HashMap::new();
        for item in users_shadow_arc.iter() { users_shadow.insert(*item.key(), item.value().clone()); }

        users_shadow
    }

    pub async fn clear_users_shadow(&self)
    {
        let users_shadow_arc = self.users_shadow.clone();

        users_shadow_arc.clear();

        self.set_stats(StatsEvent::UsersShadow, 0).await;
    }
}