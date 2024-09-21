use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::SystemTime;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl TorrentTracker {
    pub fn add_user_update(&self, user_id: UserId, user_entry_item: UserEntryItem) -> (UserEntryItem, bool)
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        match lock.insert(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(), (user_id, user_entry_item.clone())) {
            None => {
                self.update_stats(StatsEvent::UsersUpdates, 1);
                (user_entry_item, true)
            }
            Some(_) => {
                (user_entry_item, false)
            }
        }
    }

    pub fn get_user_updates(&self) -> HashMap<u128, (UserId, UserEntryItem)>
    {
        let map = self.users_updates.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

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

    pub fn clear_user_updates(&self)
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        lock.clear();
        self.set_stats(StatsEvent::UsersUpdates, 0);
    }

    pub async fn save_user_updates(&self, torrent_tracker: Arc<TorrentTracker>)
    {
        let mut hashmapping: HashMap<UserId, (Vec<u128>, UserEntryItem)> = HashMap::new();
        let mut hashmap: BTreeMap<UserId, UserEntryItem> = BTreeMap::new();
        let updates = self.get_user_updates();

        // Build the actually updates for SQL, adding the timestamps into a vector for removal afterward.
        for (timestamp, (user_id, user_entry_item)) in updates.iter() {
            match hashmapping.get_mut(user_id) {
                None => {
                    hashmapping.insert(*user_id, (vec![*timestamp], user_entry_item.clone()));
                    hashmap.insert(*user_id, user_entry_item.clone());
                }
                Some((timestamps, _)) => {
                    if !timestamps.contains(timestamp) {
                        timestamps.push(*timestamp);
                    }
                    hashmap.insert(*user_id, user_entry_item.clone());
                }
            }
        }

        // Now we're going to save the torrents in a list, and depending on what we get returned, we remove them from the updates list.
        if self.save_users(torrent_tracker.clone(), hashmap).await.is_ok() {
            // We can remove the updates keys, since they are updated.
            for (_, (timestamps, _)) in hashmapping.iter() {
                for timestamp in timestamps.iter() {
                    self.remove_user_update(timestamp);
                }
            }
        }
    }
}
