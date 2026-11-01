use crate::cache::structs::cache_connector::CacheConnector;
use crate::cache::structs::user_stat_counts::UserStatCounts;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::impls::torrent_tracker_torrents_updates::next_seq;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{
    debug,
    error,
    info,
    warn
};
use std::collections::{
    BTreeMap,
    HashMap
};
use std::sync::Arc;

impl TorrentTracker {
    /// Queues a user change for the next database/cache flush, replacing any change still queued for
    /// that user.
    ///
    /// Returns the queued entry and `true` when the user had nothing queued yet.
    pub fn add_user_update(&self, user_id: UserId, user_entry_item: UserEntryItem, updates_action: UpdatesAction) -> (UserEntryItem, bool)
    {
        let mut lock = self.users_updates.write();
        if lock.insert(user_id, (next_seq(), user_entry_item.clone(), updates_action)).is_none() {
            self.update_stats(StatsEvent::UsersUpdates, 1);
            (user_entry_item, true)
        } else {
            (user_entry_item, false)
        }
    }

    /// Returns a clone of the pending user-update queue.
    pub fn get_user_updates(&self) -> HashMap<UserId, (u128, UserEntryItem, UpdatesAction)>
    {
        let lock = self.users_updates.read();
        lock.clone()
    }

    /// Drops all queued user updates and resets the queue statistic.
    pub fn clear_user_updates(&self)
    {
        let mut lock = self.users_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::UsersUpdates, 0);
    }

    /// Flushes a snapshot of the user-update queue to the database (when the users table is
    /// persistent) and to the cache, removing the flushed entries only after the database write
    /// succeeds.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database flush fails; the queued updates remain intact.
    pub async fn save_user_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), sqlx::Error>
    {
        let updates = {
            let lock = self.users_updates.read();
            lock.clone()
        };
        if updates.is_empty() {
            return Ok(());
        }
        let mapping_len = updates.len();
        let users_to_save: BTreeMap<UserId, (UserEntryItem, UpdatesAction)> = updates
            .iter()
            .map(|(user_id, (_, user_entry_item, updates_action))| (*user_id, (user_entry_item.clone(), *updates_action)))
            .collect();
        // Cached first, from the same snapshot: `save_users` takes it by value. As with torrents
        // the cache does not wait on the database, and the totals are absolute, so a batch the
        // database rejects is simply written to the cache again on the next flush.
        if let Some(ref cache) = self.cache {
            self.cache_user_updates(cache, &users_to_save).await;
        }
        let is_persistent = torrent_tracker.config.database_structure.users.persistent.unwrap_or(torrent_tracker.config.database.persistent);
        let db_result = if is_persistent {
            self.sqlx.save_users(torrent_tracker, users_to_save).await
        } else {
            Ok(())
        };
        match db_result {
            Ok(()) => {
                if is_persistent {
                    info!("[SYNC USER UPDATES] Synced {mapping_len} users");
                }
                self.retire_flushed(&self.users_updates, &updates, |(seq, _, _)| *seq, StatsEvent::UsersUpdates);
                Ok(())
            }
            Err(e) => {
                error!("[SYNC USER UPDATES] Unable to sync {mapping_len} users: {e}");
                Err(e)
            }
        }
    }

    /// Writes the users' totals to the cache (`<prefix>u:<id>`) and deletes the removed ones.
    /// A failure is only logged, as for torrents: the cache is a copy and the next flush rewrites it.
    async fn cache_user_updates(&self, cache: &CacheConnector, users: &BTreeMap<UserId, (UserEntryItem, UpdatesAction)>)
    {
        let mut writes: Vec<(String, UserStatCounts)> = Vec::new();
        let mut removals: Vec<String> = Vec::new();
        for (user, action) in users.values() {
            let Some(id) = user.cache_id() else {
                warn!("[Cache] Skipping user update with no id or uuid set");
                continue;
            };
            if *action == UpdatesAction::Remove {
                removals.push(id);
            } else {
                writes.push((id, UserStatCounts::from(user)));
            }
        }
        match cache.set_users_batch(&writes).await {
            Ok(()) => debug!("[Cache] Updated {} users", writes.len()),
            Err(e) => warn!("[Cache] Failed to update {} users: {e}", writes.len()),
        }
        if let Err(e) = cache.delete_users(&removals).await {
            warn!("[Cache] Failed to delete {} users: {e}", removals.len());
        }
    }
}
