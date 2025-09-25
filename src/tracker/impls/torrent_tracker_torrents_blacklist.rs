use std::sync::Arc;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(blacklist) = self.sqlx.load_blacklist(tracker).await {
            info!("Loaded {blacklist} blacklists");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<(InfoHash, UpdatesAction)>) -> Result<(), ()>
    {
        let hashes_len = hashes.len();
        match self.sqlx.save_blacklist(tracker, hashes).await {
            Ok(_) => {
                info!("[SYNC BLACKLIST] Synced {hashes_len} blacklists");
                Ok(())
            }
            Err(_) => {
                error!("[SYNC BLACKLIST] Unable to sync {hashes_len} blacklists");
                Err(())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let mut lock = self.torrents_blacklist.write();
        if !lock.contains(&info_hash) {
            lock.push(info_hash);
            self.update_stats(StatsEvent::Blacklist, 1);
            return true;
        }
        false
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let lock = self.torrents_blacklist.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let lock = self.torrents_blacklist.read_recursive();
        lock.contains(&info_hash)
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let mut lock = self.torrents_blacklist.write();
        if let Some(index) = lock.iter().position(|r| *r == info_hash) {
            lock.swap_remove(index);
            self.update_stats(StatsEvent::Blacklist, -1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_blacklist(&self)
    {
        let mut lock = self.torrents_blacklist.write();
        lock.clear();
    }
}