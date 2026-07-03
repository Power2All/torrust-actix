use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use std::sync::Arc;

impl TorrentTracker {
    /// Loads the whitelist from the configured database into memory at startup.
    ///
    /// A load failure is logged and leaves the in-memory whitelist empty.
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>)
    {
        match self.sqlx.load_whitelist(tracker).await {
            Ok(whitelist) => {
                info!("Loaded {whitelist} whitelists");
            }
            Err(e) => {
                error!("Unable to load the whitelist from the database: {e}");
            }
        }
    }

    /// Persists whitelist additions/removals to the database.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database write fails; the caller re-queues the batch.
    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<(InfoHash, UpdatesAction)>) -> Result<(), ()>
    {
        let hashes_len = hashes.len();
        if self.sqlx.save_whitelist(tracker, hashes).await.is_ok() {
            info!("[SYNC WHITELIST] Synced {hashes_len} whitelists");
            Ok(())
        } else {
            error!("[SYNC WHITELIST] Unable to sync {hashes_len} whitelists");
            Err(())
        }
    }

    /// Adds an info-hash to the whitelist; returns `true` when it was newly inserted.
    #[inline]
    pub fn add_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let mut lock = self.torrents_whitelist.write();
        if lock.insert(info_hash) {
            self.update_stats(StatsEvent::Whitelist, 1);
            return true;
        }
        false
    }

    /// Returns all whitelisted info-hashes.
    pub fn get_whitelist(&self) -> Vec<InfoHash>
    {
        let lock = self.torrents_whitelist.read();
        lock.iter().copied().collect()
    }

    /// Returns `true` when the info-hash is whitelisted (checked on every announce when
    /// whitelist mode is enabled).
    #[inline]
    pub fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let lock = self.torrents_whitelist.read();
        lock.contains(&info_hash)
    }

    /// Removes an info-hash from the whitelist; returns `true` when it existed.
    #[inline]
    pub fn remove_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let mut lock = self.torrents_whitelist.write();
        if lock.remove(&info_hash) {
            self.update_stats(StatsEvent::Whitelist, -1);
            true
        } else {
            false
        }
    }

    /// Removes all whitelist entries and resets the whitelist counter statistic.
    pub fn clear_whitelist(&self)
    {
        let mut lock = self.torrents_whitelist.write();
        lock.clear();
        self.set_stats(StatsEvent::Whitelist, 0);
    }
}