use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::SystemTime;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl TorrentTracker {
    #[tracing::instrument]
    pub fn add_user_update(&self, user_id: UserId, user_entry_item: UserEntryItem, updates_action: UpdatesAction) -> (UserEntryItem, bool)
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        match lock.insert(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(), (user_id, user_entry_item.clone(), updates_action)) {
            None => {
                self.update_stats(StatsEvent::UsersUpdates, 1);
                (user_entry_item, true)
            }
            Some(_) => {
                (user_entry_item, false)
            }
        }
    }

    #[tracing::instrument]
    pub fn get_user_updates(&self) -> HashMap<u128, (UserId, UserEntryItem, UpdatesAction)>
    {
        let map = self.users_updates.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    #[tracing::instrument]
    pub fn remove_user_update(&self, timestamp: &u128) -> bool
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        match lock.remove(timestamp) {
            None => { false }
            Some(_) => {
                self.update_stats(StatsEvent::UsersUpdates, -1);
                true
            }
        }
    }

    #[tracing::instrument]
    pub fn clear_user_updates(&self)
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        lock.clear();
        self.set_stats(StatsEvent::UsersUpdates, 0);
    }

    #[tracing::instrument]
    pub async fn save_user_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let mut mapping: HashMap<UserId, (u128, UserEntryItem, UpdatesAction)> = HashMap::new();
        for (timestamp, (user_id, user_entry_item, updates_action)) in self.get_user_updates().iter() {
            match mapping.entry(*user_id) {
                Entry::Occupied(mut o) => {
                    o.insert((o.get().0, user_entry_item.clone(), *updates_action));
                    self.remove_user_update(timestamp);
                }
                Entry::Vacant(v) => {
                    v.insert((*timestamp, user_entry_item.clone(), *updates_action));
                }
            }
        }
        match self.save_users(torrent_tracker.clone(), mapping.clone().into_iter().map(|(user_id, (_, user_entry_item, updates_action))| {
            (user_id, (user_entry_item.clone(), updates_action))
        }).collect::<BTreeMap<UserId, (UserEntryItem, UpdatesAction)>>()).await {
            Ok(_) => {
                info!("[SYNC USER UPDATES] Synced {} users", mapping.len());
                for (_, (timestamp, _, _)) in mapping.into_iter() {
                    self.remove_user_update(&timestamp);
                }
                Ok(())
            }
            Err(_) => {
                error!("[SYNC USER UPDATES] Unable to sync {} users", mapping.len());
                Err(())
            }
        }
    }
}