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
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(blacklist) = self.sqlx.load_blacklist(tracker).await {
            info!("Loaded {blacklist} blacklists");
        }
    }

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

    pub fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let lock = self.torrents_blacklist.read();
        lock.iter().copied().collect()
    }

    #[inline]
    pub fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let lock = self.torrents_blacklist.read();
        lock.contains(&info_hash)
    }

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

    pub fn clear_blacklist(&self)
    {
        let mut lock = self.torrents_blacklist.write();
        lock.clear();
    }
}