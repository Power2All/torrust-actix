use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::impls::torrent_tracker_torrents_updates::next_seq;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use std::collections::BTreeMap;
use std::sync::Arc;

impl TorrentTracker {
    /// Queues an announce-key change (with expiry) for the next database flush, replacing any
    /// change still queued for that key; returns `true` when it had nothing queued yet.
    pub fn add_key_update(&self, info_hash: InfoHash, timeout: i64, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.keys_updates.write();
        if lock.insert(info_hash, (next_seq(), timeout, updates_action)).is_none() {
            self.update_stats(StatsEvent::KeyUpdates, 1);
            true
        } else {
            false
        }
    }

    /// Drops all queued key updates and resets the queue statistic.
    pub fn clear_key_updates(&self)
    {
        let mut lock = self.keys_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::KeyUpdates, 0);
    }

    /// Flushes a snapshot of the key-update queue to the database, removing the flushed entries
    /// only after the write succeeds.
    ///
    /// # Errors
    ///
    /// Returns the database error when the flush fails; the queued updates remain intact.
    pub async fn save_key_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), sqlx::Error>
    {
        let updates = self.keys_updates.read_recursive().clone();
        if updates.is_empty() {
            return Ok(());
        }
        let mapping_len = updates.len();
        let keys_to_save: BTreeMap<InfoHash, (i64, UpdatesAction)> = updates
            .iter()
            .map(|(info_hash, (_, timeout, updates_action))| (*info_hash, (*timeout, *updates_action)))
            .collect();
        match self.sqlx.save_keys(torrent_tracker, keys_to_save).await {
            Ok(_) => {
                info!("[SYNC KEY UPDATES] Synced {mapping_len} keys");
                self.retire_flushed(&self.keys_updates, &updates, |(seq, _, _)| *seq, StatsEvent::KeyUpdates);
                Ok(())
            }
            Err(e) => {
                error!("[SYNC KEY UPDATES] Unable to sync {mapping_len} keys: {e}");
                Err(e)
            }
        }
    }
}
