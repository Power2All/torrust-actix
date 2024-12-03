use std::sync::Arc;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(whitelist) = self.sqlx.load_whitelist(tracker.clone()).await {
            info!("Loaded {} whitelists", whitelist);
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<(InfoHash, UpdatesAction)>) -> Result<(), ()>
    {
        match self.sqlx.save_whitelist(tracker.clone(), hashes.clone()).await {
            Ok(_) => {
                info!("[SYNC WHITELIST] Synced {} whitelists", hashes.len());
                Ok(())
            }
            Err(_) => {
                error!("[SYNC WHITELIST] Unable to sync {} whitelists", hashes.len());
                Err(())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_whitelist.clone();
        let mut lock = map.write();
        if !lock.contains(&info_hash) {
            lock.push(info_hash);
            self.update_stats(StatsEvent::Whitelist, 1);
            return true;
        }
        false
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_whitelist(&self) -> Vec<InfoHash>
    {
        let map = self.torrents_whitelist.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_whitelist.clone();
        let lock = map.read_recursive();
        if lock.contains(&info_hash) {
            return true;
        }
        false
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_whitelist.clone();
        let mut lock = map.write();
        match lock.iter().position(|r| *r == info_hash) {
            None => { false }
            Some(index) => {
                lock.remove(index);
                self.update_stats(StatsEvent::Whitelist, -1);
                true
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_whitelist(&self)
    {
        let map = self.torrents_whitelist.clone();
        let mut lock = map.write();
        lock.clear();
    }
}