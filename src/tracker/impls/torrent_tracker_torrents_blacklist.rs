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
    /// Loads the blacklist from the configured database into memory at startup.
    ///
    /// A load failure is logged and leaves the in-memory blacklist empty.
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>)
    {
        match self.sqlx.load_blacklist(tracker).await {
            Ok(blacklist) => {
                info!("Loaded {blacklist} blacklists");
            }
            Err(e) => {
                error!("Unable to load the blacklist from the database: {e}");
            }
        }
    }

    /// Persists blacklist additions/removals to the database.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database write fails; the caller re-queues the batch.
    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<(InfoHash, UpdatesAction)>) -> Result<(), ()>
    {
        let hashes_len = hashes.len();
        if self.sqlx.save_blacklist(tracker, hashes).await.is_ok() {
            info!("[SYNC BLACKLIST] Synced {hashes_len} blacklists");
            Ok(())
        } else {
            error!("[SYNC BLACKLIST] Unable to sync {hashes_len} blacklists");
            Err(())
        }
    }

    /// Adds an info-hash to the blacklist; returns `true` when it was newly inserted.
    #[inline]
    pub fn add_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let mut lock = self.torrents_blacklist.write();
        if lock.insert(info_hash) {
            self.update_stats(StatsEvent::Blacklist, 1);
            return true;
        }
        false
    }

    /// Returns all blacklisted info-hashes.
    pub fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let lock = self.torrents_blacklist.read();
        lock.iter().copied().collect()
    }

    /// Returns `true` when the info-hash is blacklisted (checked on every announce when
    /// blacklist mode is enabled).
    #[inline]
    pub fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let lock = self.torrents_blacklist.read();
        lock.contains(&info_hash)
    }

    /// Removes an info-hash from the blacklist; returns `true` when it existed.
    #[inline]
    pub fn remove_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let mut lock = self.torrents_blacklist.write();
        if lock.remove(&info_hash) {
            self.update_stats(StatsEvent::Blacklist, -1);
            true
        } else {
            false
        }
    }

    /// Removes all blacklist entries and resets the blacklist counter statistic.
    pub fn clear_blacklist(&self)
    {
        let mut lock = self.torrents_blacklist.write();
        lock.clear();
        self.set_stats(StatsEvent::Blacklist, 0);
    }
}