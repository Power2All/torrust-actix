use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{error, info};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(users) = self.sqlx.load_users(tracker).await {
            info!("Loaded {users} users");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>) -> Result<(), ()>
    {
        let users_len = users.len();
        match self.sqlx.save_users(tracker, users).await {
            Ok(_) => {
                info!("[SYNC USERS] Synced {users_len} users");
                Ok(())
            }
            Err(_) => {
                error!("[SYNC USERS] Unable to sync {users_len} users");
                Err(())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_user(&self, user_id: UserId, user_entry_item: UserEntryItem) -> bool
    {
        let mut lock = self.users.write();
        match lock.entry(user_id) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Users, 1);
                v.insert(user_entry_item);
                true
            }
            Entry::Occupied(mut o) => {
                o.insert(user_entry_item);
                false
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_user_active_torrent(&self, user_id: UserId, info_hash: InfoHash) -> bool
    {
        let mut lock = self.users.write();
        match lock.entry(user_id) {
            Entry::Vacant(_) => {
                false
            }
            Entry::Occupied(mut o) => {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                o.get_mut().torrents_active.insert(info_hash, timestamp);
                true
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_user(&self, id: UserId) -> Option<UserEntryItem>
    {
        let lock = self.users.read_recursive();
        lock.get(&id).cloned()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_users(&self) -> BTreeMap<UserId, UserEntryItem>
    {
        let lock = self.users.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_user(&self, user_id: UserId) -> Option<UserEntryItem>
    {
        let mut lock = self.users.write();
        if let Some(data) = lock.remove(&user_id) {
            self.update_stats(StatsEvent::Users, -1);
            Some(data)
        } else {
            None
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_user_active_torrent(&self, user_id: UserId, info_hash: InfoHash) -> bool
    {
        let mut lock = self.users.write();
        match lock.entry(user_id) {
            Entry::Vacant(_) => {
                false
            }
            Entry::Occupied(mut o) => {
                o.get_mut().torrents_active.remove(&info_hash).is_some()
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn check_user_key(&self, key: UserId) -> Option<UserId>
    {
        let lock = self.users.read_recursive();
        for (user_id, user_entry_item) in lock.iter() {
            if user_entry_item.key == key {
                return Some(*user_id);
            }
        }
        None
    }

    #[tracing::instrument(level = "debug")]
    pub fn clean_user_active_torrents(&self, peer_timeout: Duration)
    {
        let current_time = SystemTime::now();
        let timeout_threshold = current_time.duration_since(UNIX_EPOCH).unwrap().as_secs() - peer_timeout.as_secs();
        let remove_active_torrents = {
            let lock = self.users.read_recursive();
            info!("[USERS] Scanning {} users with dead active torrents", lock.len());
            let mut to_remove = Vec::new();
            for (user_id, user_entry_item) in lock.iter() {
                for (info_hash, &updated) in &user_entry_item.torrents_active {
                    if updated < timeout_threshold {
                        to_remove.push((*user_id, *info_hash));
                    }
                }
            }
            to_remove
        };
        let torrents_cleaned = remove_active_torrents.len() as u64;
        if !remove_active_torrents.is_empty() {
            let mut lock = self.users.write();
            for (user_id, info_hash) in remove_active_torrents {
                if let Entry::Occupied(mut o) = lock.entry(user_id) {
                    o.get_mut().torrents_active.remove(&info_hash);
                }
            }
        }
        info!("[USERS] Removed {torrents_cleaned} active torrents in users");
    }
}