use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl TorrentTracker {
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(users) = self.sqlx.load_users(tracker.clone()).await {
            info!("Loaded {} users", users);
        }
    }

    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, UserEntryItem>) -> Result<(), ()>
    {
        match self.sqlx.save_users(tracker.clone(), users.clone()).await {
            Ok(_) => {
                info!("[SAVE USERS] Saved {} users", users.len());
                Ok(())
            }
            Err(_) => {
                error!("[SAVE USERS] Unable to save {} users", users.len());
                Err(())
            }
        }
    }

    pub fn get_user(&self, id: UserId) -> Option<UserEntryItem>
    {
        let map = self.users.clone();
        let lock = map.read_recursive();
        lock.get(&id).cloned()
    }

    pub fn get_users(&self) -> BTreeMap<UserId, UserEntryItem>
    {
        let map = self.users.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    pub fn add_user(&self, user_id: UserId, user_entry_item: UserEntryItem) -> bool
    {
        let map = self.users.clone();
        let mut lock = map.write();
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

    pub fn add_user_active_torrent(&self, user_id: UserId, info_hash: InfoHash) -> bool
    {
        let map = self.users.clone();
        let mut lock = map.write();
        match lock.entry(user_id) {
            Entry::Vacant(_) => {
                false
            }
            Entry::Occupied(mut o) => {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let timestamp_unix = timestamp.as_secs();
                o.get_mut().torrents_active.insert(info_hash, timestamp_unix);
                true
            }
        }
    }

    pub fn remove_user(&self, user_id: UserId) -> Option<UserEntryItem>
    {
        let map = self.users.clone();
        let mut lock = map.write();
        match lock.remove(&user_id) {
            None => { None }
            Some(data) => {
                self.update_stats(StatsEvent::Users, -1);
                Some(data)
            }
        }
    }

    pub fn remove_user_active_torrent(&self, user_id: UserId, info_hash: InfoHash) -> bool
    {
        let map = self.users.clone();
        let mut lock = map.write();
        match lock.entry(user_id) {
            Entry::Vacant(_) => {
                false
            }
            Entry::Occupied(mut o) => {
                match o.get_mut().torrents_active.remove(&info_hash) {
                    None => { false }
                    Some(_) => { true }
                }
            }
        }
    }

    pub fn check_user_key(&self, key: UserId) -> Option<UserId>
    {
        let map = self.users.clone();
        let lock = map.read_recursive();
        for (user_id, user_entry_item) in lock.iter() {
            if user_entry_item.key == key {
                return Some(*user_id);
            }
        }
        None
    }

    pub fn clean_user_active_torrents(&self, peer_timeout: Duration)
    {
        let mut torrents_cleaned = 0u64;
        let mut remove_active_torrents = vec![];
        let map = self.users.clone();
        let lock = map.read_recursive();
        info!("[USERS] Scanning {} users with dead active torrents", lock.len());
        for (user_id, user_entry_item) in lock.iter() {
            let torrents_active = user_entry_item.torrents_active.clone();
            for (info_hash, updated) in torrents_active.iter() {
                let time = SystemTime::from(Utc.timestamp_opt(*updated as i64 + peer_timeout.as_secs() as i64, 0).unwrap());
                if time.duration_since(SystemTime::now()).is_err() {
                    remove_active_torrents.push((*user_id, *info_hash));
                }
            }
        }
        for (user_id, info_hash) in remove_active_torrents {
            self.remove_user_active_torrent(user_id, info_hash);
            torrents_cleaned += 1;
        }
        info!("[USERS] Removed {} active torrents in users", torrents_cleaned);
    }
}
