use std::sync::Arc;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(whitelist) = self.sqlx.load_whitelist(tracker.clone()).await {
            info!("Loaded {} whitelists", whitelist);
        }
    }

    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, hashes: Vec<InfoHash>) -> Result<(), ()>
    {
        match self.sqlx.save_whitelist(tracker.clone(), hashes.clone()).await {
            Ok(_) => {
                info!("[SAVE WHITELIST] Saved {} whitelists", hashes.len());
                Ok(())
            }
            Err(_) => {
                error!("[SAVE WHITELIST] Unable to save {} whitelists", hashes.len());
                Err(())
            }
        }
    }

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

    pub fn get_whitelist(&self) -> Vec<InfoHash>
    {
        let map = self.torrents_whitelist.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    pub fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let map = self.torrents_whitelist.clone();
        let lock = map.read_recursive();
        if lock.contains(&info_hash) {
            return true;
        }
        false
    }

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

    pub fn clear_whitelist(&self)
    {
        let map = self.torrents_whitelist.clone();
        let mut lock = map.write();
        lock.clear();
    }
}
