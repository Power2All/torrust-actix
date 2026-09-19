use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{
    error,
    info
};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

impl TorrentTracker {
    /// Loads all users from the configured database into memory at startup.
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(users) = self.sqlx.load_users(tracker).await {
            info!("Loaded {users} users");
        }
    }

    /// Persists the given batch of user entries (and their add/remove actions) to the database.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database write fails; the caller is expected to re-queue the batch.
    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>) -> Result<(), ()>
    {
        let users_len = users.len();
        if let Ok(()) = self.sqlx.save_users(tracker, users).await {
            info!("[SYNC USERS] Synced {users_len} users");
            Ok(())
        } else {
            error!("[SYNC USERS] Unable to sync {users_len} users");
            Err(())
        }
    }

    /// Inserts or replaces a user entry and keeps the key -> user-id index in sync.
    ///
    /// Returns `true` when the user was newly inserted, `false` when an existing entry was replaced.
    pub fn add_user(&self, user_id: UserId, user_entry_item: UserEntryItem) -> bool
    {
        let mut lock = self.users.write();
        let mut index = self.users_key_index.write();
        match lock.entry(user_id) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Users, 1);
                index.insert(user_entry_item.key, user_id);
                v.insert(user_entry_item);
                true
            }
            Entry::Occupied(mut o) => {
                let old_key = o.get().key;
                if old_key != user_entry_item.key && index.get(&old_key) == Some(&user_id) {
                    index.remove(&old_key);
                }
                index.insert(user_entry_item.key, user_id);
                o.insert(user_entry_item);
                false
            }
        }
    }

    /// Returns a clone of the user entry for the given [`UserId`], if present.
    pub fn get_user(&self, id: UserId) -> Option<UserEntryItem>
    {
        let lock = self.users.read_recursive();
        lock.get(&id).cloned()
    }

    /// Returns a clone of the complete user table.
    ///
    /// This copies every entry; intended for the API and persistence tasks, not per-request use.
    pub fn get_users(&self) -> BTreeMap<UserId, UserEntryItem>
    {
        let lock = self.users.read_recursive();
        lock.clone()
    }

    /// Removes a user (and its key-index entry), returning the removed entry if it existed.
    pub fn remove_user(&self, user_id: UserId) -> Option<UserEntryItem>
    {
        let mut lock = self.users.write();
        if let Some(data) = lock.remove(&user_id) {
            let mut index = self.users_key_index.write();
            if index.get(&data.key) == Some(&user_id) {
                index.remove(&data.key);
            }
            self.update_stats(StatsEvent::Users, -1);
            Some(data)
        } else {
            None
        }
    }

    /// Removes all users and resets the user counter statistic.
    pub fn clear_users(&self)
    {
        let mut lock = self.users.write();
        lock.clear();
        self.users_key_index.write().clear();
        self.set_stats(StatsEvent::Users, 0);
    }

    /// Removes a torrent from the user's active-torrent list.
    ///
    /// Returns `true` when the torrent was present and removed.
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

    /// Resolves a user announce key to its [`UserId`] via the O(1) key index.
    ///
    /// Returns `None` when no user owns the given key, or when the owning user is not active.
    /// The `active` flag is the tracker's only way to turn a user off, so a deactivated user
    /// must stop resolving here rather than merely being reported as inactive by the API.
    pub fn check_user_key(&self, key: UserId) -> Option<UserId>
    {
        let user_id = {
            let lock = self.users_key_index.read_recursive();
            lock.get(&key).copied()?
        };
        let users = self.users.read_recursive();
        users.get(&user_id).filter(|user| user.active != 0).map(|_| user_id)
    }

    /// Mutates a user entry in place under a single write lock, avoiding the
    /// `get_user` (clone) -> mutate -> `add_user` (clone) round-trip on the announce path.
    ///
    /// The closure must not modify `UserEntryItem::key`, as the key index is not updated here.
    /// Returns a clone of the updated entry when `return_clone` is true (used to enqueue a
    /// persistence update), or `None` when the user does not exist or no clone was requested.
    pub fn update_user_on_announce<F>(&self, user_id: UserId, return_clone: bool, mutate: F) -> Option<UserEntryItem>
    where
        F: FnOnce(&mut UserEntryItem)
    {
        let mut lock = self.users.write();
        let user = lock.get_mut(&user_id)?;
        mutate(user);
        if return_clone { Some(user.clone()) } else { None }
    }

}