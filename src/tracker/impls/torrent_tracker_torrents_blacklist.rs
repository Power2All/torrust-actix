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
        if let Ok(blacklist) = self.sqlx.load_blacklist(tracker.clone()).await {
            info!("Loaded {blacklist} blacklists");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<(InfoHash, UpdatesAction)>) -> Result<(), ()>
    {
        match self.sqlx.save_blacklist(tracker.clone(), hashes.clone()).await {
            Ok(_) => {
                info!("[SYNC BLACKLIST] Synced {} blacklists", hashes.len());
                Ok(())
            }
            Err(_) => {
                error!("[SYNC BLACKLIST] Unable to sync {} blacklists", hashes.len());
                Err(())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_blacklist.clone();
        let mut lock = map.write();
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
        let map = self.torrents_blacklist.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_blacklist.clone();
        let lock = map.read_recursive();
        if lock.contains(&info_hash) {
            return true;
        }
        false
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_blacklist.clone();
        let mut lock = map.write();
        match lock.iter().position(|r| *r == info_hash) {
            None => { false }
            Some(index) => {
                lock.remove(index);
                self.update_stats(StatsEvent::Blacklist, -1);
                true
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_blacklist(&self)
    {
        let map = self.torrents_blacklist.clone();
        let mut lock = map.write();
        lock.clear();
    }
}