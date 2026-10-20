use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::impls::torrent_tracker_torrents_updates::next_seq;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{
    error,
    info
};
use std::collections::{
    BTreeMap,
    HashMap
};
use std::sync::Arc;

impl TorrentTracker {
    /// Queues a user change for the next database flush, replacing any change still queued for
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
        let lock = self.users_updates.read_recursive();
        lock.clone()
    }

    /// Drops all queued user updates and resets the queue statistic.
    pub fn clear_user_updates(&self)
    {
        let mut lock = self.users_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::UsersUpdates, 0);
    }

    /// Flushes a snapshot of the user-update queue to the database, removing the flushed entries
    /// only after the write succeeds.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the flush fails; the queued updates remain intact.
    pub async fn save_user_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), sqlx::Error>
    {
        let updates = {
            let lock = self.users_updates.read_recursive();
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
        match self.sqlx.save_users(torrent_tracker, users_to_save).await {
            Ok(()) => {
                info!("[SYNC USER UPDATES] Synced {mapping_len} users");
                self.retire_flushed(&self.users_updates, &updates, |(seq, _, _)| *seq, StatsEvent::UsersUpdates);
                Ok(())
            }
            Err(e) => {
                error!("[SYNC USER UPDATES] Unable to sync {mapping_len} users: {e}");
                Err(e)
            }
        }
    }
}
