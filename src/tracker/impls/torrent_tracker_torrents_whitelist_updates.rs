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
    /// Queues a whitelist change for the next database flush, replacing any change still queued for
    /// that info-hash; returns `true` when it had nothing queued yet.
    pub fn add_whitelist_update(&self, info_hash: InfoHash, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.torrents_whitelist_updates.write();
        if lock.insert(info_hash, (next_seq(), updates_action)).is_none() {
            self.update_stats(StatsEvent::WhitelistUpdates, 1);
            true
        } else {
            false
        }
    }

    /// Drops all queued whitelist updates and resets the queue statistic.
    pub fn clear_whitelist_updates(&self)
    {
        let mut lock = self.torrents_whitelist_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::WhitelistUpdates, 0);
    }

    /// Flushes a snapshot of the whitelist-update queue to the database, removing the flushed entries
    /// only after the write succeeds.
    ///
    /// # Errors
    ///
    /// Returns the database error when the flush fails; the queued updates remain intact.
    pub async fn save_whitelist_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), sqlx::Error>
    {
        let updates = self.torrents_whitelist_updates.read().clone();
        if updates.is_empty() {
            return Ok(());
        }
        let mapping_len = updates.len();
        let whitelist_updates: Vec<(InfoHash, UpdatesAction)> = updates
            .iter()
            .map(|(info_hash, (_, updates_action))| (*info_hash, *updates_action))
            .collect();
        match self.sqlx.save_whitelist(torrent_tracker, whitelist_updates).await {
            Ok(_) => {
                info!("[SYNC WHITELIST UPDATES] Synced {mapping_len} whitelists");
                self.retire_flushed(&self.torrents_whitelist_updates, &updates, |(seq, _)| *seq, StatsEvent::WhitelistUpdates);
                Ok(())
            }
            Err(e) => {
                error!("[SYNC WHITELIST UPDATES] Unable to sync {mapping_len} whitelists: {e}");
                Err(e)
            }
        }
    }
}
