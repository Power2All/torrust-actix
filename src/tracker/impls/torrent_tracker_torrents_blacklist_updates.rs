use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::impls::torrent_tracker_torrents_updates::next_seq;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use std::sync::Arc;

impl TorrentTracker {
    /// Queues a blacklist change for the next database flush, replacing any change still queued for
    /// that info-hash; returns `true` when it had nothing queued yet.
    pub fn add_blacklist_update(&self, info_hash: InfoHash, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.torrents_blacklist_updates.write();
        if lock.insert(info_hash, (next_seq(), updates_action)).is_none() {
            self.update_stats(StatsEvent::BlacklistUpdates, 1);
            true
        } else {
            false
        }
    }

    /// Drops all queued blacklist updates and resets the queue statistic.
    pub fn clear_blacklist_updates(&self)
    {
        let mut lock = self.torrents_blacklist_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::BlacklistUpdates, 0);
    }

    /// Flushes a snapshot of the blacklist-update queue to the database, removing the flushed entries
    /// only after the write succeeds.
    ///
    /// # Errors
    ///
    /// Returns the database error when the flush fails; the queued updates remain intact.
    pub async fn save_blacklist_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), sqlx::Error>
    {
        let updates = self.torrents_blacklist_updates.read_recursive().clone();
        if updates.is_empty() {
            return Ok(());
        }
        let mapping_len = updates.len();
        let blacklist_updates: Vec<(InfoHash, UpdatesAction)> = updates
            .iter()
            .map(|(info_hash, (_, updates_action))| (*info_hash, *updates_action))
            .collect();
        match self.sqlx.save_blacklist(torrent_tracker, blacklist_updates).await {
            Ok(_) => {
                info!("[SYNC BLACKLIST UPDATES] Synced {mapping_len} blacklists");
                self.retire_flushed(&self.torrents_blacklist_updates, &updates, |(seq, _)| *seq, StatsEvent::BlacklistUpdates);
                Ok(())
            }
            Err(e) => {
                error!("[SYNC BLACKLIST UPDATES] Unable to sync {mapping_len} blacklists: {e}");
                Err(e)
            }
        }
    }
}
