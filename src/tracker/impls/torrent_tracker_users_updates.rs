use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{
    error,
    info
};
use std::collections::hash_map::Entry;
use std::collections::{
    BTreeMap,
    HashMap
};
use std::sync::Arc;
use std::time::SystemTime;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub fn add_user_update(&self, user_id: UserId, user_entry_item: UserEntryItem, updates_action: UpdatesAction) -> (UserEntryItem, bool)
    {
        let mut lock = self.users_updates.write();
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        if lock.insert(timestamp, (user_id, user_entry_item.clone(), updates_action)).is_none() {
            self.update_stats(StatsEvent::UsersUpdates, 1);
            (user_entry_item, true)
        } else {
            (user_entry_item, false)
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_user_updates(&self) -> HashMap<u128, (UserId, UserEntryItem, UpdatesAction)>
    {
        let lock = self.users_updates.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_user_update(&self, timestamp: &u128) -> bool
    {
        let mut lock = self.users_updates.write();
        if lock.remove(timestamp).is_some() {
            self.update_stats(StatsEvent::UsersUpdates, -1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_user_updates(&self)
    {
        let mut lock = self.users_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::UsersUpdates, 0);
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_user_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let updates = {
            let lock = self.users_updates.read_recursive();
            lock.clone()
        };
        if updates.is_empty() {
            return Ok(());
        }
        let mut mapping: HashMap<UserId, (u128, UserEntryItem, UpdatesAction)> = HashMap::with_capacity(updates.len());
        let mut timestamps_to_remove = Vec::new();
        for (timestamp, (user_id, user_entry_item, updates_action)) in updates {
            match mapping.entry(user_id) {
                Entry::Occupied(mut o) => {
                    let existing = o.get();
                    if timestamp > existing.0 {
                        timestamps_to_remove.push(existing.0);
                        o.insert((timestamp, user_entry_item, updates_action));
                    } else {
                        timestamps_to_remove.push(timestamp);
                    }
                }
                Entry::Vacant(v) => {
                    v.insert((timestamp, user_entry_item, updates_action));
                }
            }
        }
        let mapping_len = mapping.len();
        let users_to_save: BTreeMap<UserId, (UserEntryItem, UpdatesAction)> = mapping
            .iter()
            .map(|(user_id, (_, user_entry_item, updates_action))| (*user_id, (user_entry_item.clone(), *updates_action)))
            .collect();
        match self.save_users(torrent_tracker, users_to_save).await {
            Ok(_) => {
                info!("[SYNC USER UPDATES] Synced {mapping_len} users");
                let mut lock = self.users_updates.write();
                let mut removed_count = 0i64;
                for (_, (timestamp, _, _)) in mapping {
                    if lock.remove(&timestamp).is_some() {
                        removed_count += 1;
                    }
                }
                for timestamp in timestamps_to_remove {
                    if lock.remove(&timestamp).is_some() {
                        removed_count += 1;
                    }
                }
                if removed_count > 0 {
                    self.update_stats(StatsEvent::UsersUpdates, -removed_count);
                }
                Ok(())
            }
            Err(_) => {
                error!("[SYNC USER UPDATES] Unable to sync {mapping_len} users");
                Err(())
            }
        }
    }
}