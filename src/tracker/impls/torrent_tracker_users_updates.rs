use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl TorrentTracker {
    pub fn get_user_update(&self, id: UserId) -> Option<UserEntryItem>
    {
        let map = self.users_updates.clone();
        let lock = map.read_recursive();
        match lock.get(&id) {
            None => {
                None
            }
            Some(user_entry_item) => {
                Some(user_entry_item.clone())
            }
        }
    }

    pub fn get_users_update(&self) -> BTreeMap<UserId, UserEntryItem>
    {
        let map = self.users_updates.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    pub fn add_user_update(&self, user_id: UserId, user_entry_item: UserEntryItem) -> bool
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        match lock.entry(user_id) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::UsersUpdates, 1);
                v.insert(user_entry_item);
                true
            }
            Entry::Occupied(mut o) => {
                o.insert(user_entry_item);
                false
            }
        }
    }

    pub fn remove_user_update(&self, user_id: UserId) -> Option<UserEntryItem>
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        match lock.remove(&user_id) {
            None => { None }
            Some(data) => {
                self.update_stats(StatsEvent::UsersUpdates, -1);
                Some(data)
            }
        }
    }

    pub fn clear_user_update(&self)
    {
        let map = self.users_updates.clone();
        let mut lock = map.write();
        self.set_stats(StatsEvent::UsersUpdates, 0);
        lock.clear();
    }
}
