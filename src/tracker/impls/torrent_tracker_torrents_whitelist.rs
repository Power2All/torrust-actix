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
    #[tracing::instrument(level = "debug")]
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(whitelist) = self.sqlx.load_whitelist(tracker).await {
            info!("Loaded {whitelist} whitelists");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<(InfoHash, UpdatesAction)>) -> Result<(), ()>
    {
        let hashes_len = hashes.len();
        match self.sqlx.save_whitelist(tracker, hashes).await {
            Ok(_) => {
                info!("[SYNC WHITELIST] Synced {hashes_len} whitelists");
                Ok(())
            }
            Err(_) => {
                error!("[SYNC WHITELIST] Unable to sync {hashes_len} whitelists");
                Err(())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
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

    #[tracing::instrument(level = "debug")]
    pub fn get_whitelist(&self) -> Vec<InfoHash>
    {
        let lock = self.torrents_whitelist.read();
        lock.iter().copied().collect()
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let lock = self.torrents_whitelist.read();
        lock.contains(&info_hash)
    }

    #[tracing::instrument(level = "debug")]
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

    #[tracing::instrument(level = "debug")]
    pub fn clear_whitelist(&self)
    {
        let mut lock = self.torrents_whitelist.write();
        lock.clear();
    }
}