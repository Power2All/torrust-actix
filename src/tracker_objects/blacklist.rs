use log::info;

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

impl TorrentTracker {
    pub async fn load_blacklists(&self)
    {
        if let Ok(blacklists) = self.sqlx.load_blacklist().await {
            let mut blacklist_count = 0i64;

            for info_hash in blacklists.iter() {
                self.add_blacklist(*info_hash, true).await;
                blacklist_count += 1;
            }

            info!("Loaded {} blacklists.", blacklist_count);
        }
    }

    pub async fn save_blacklists(&self) -> bool
    {
        let blacklist = self.get_blacklist().await;

        if self.sqlx.save_blacklist(blacklist).await.is_ok() { return true; }

        false
    }

    pub async fn add_blacklist(&self, info_hash: InfoHash, on_load: bool)
    {
        let blacklist_arc = self.blacklist.clone();

        if on_load { blacklist_arc.insert(info_hash, 1i64); } else { blacklist_arc.insert(info_hash, 2i64); }

        self.update_stats(StatsEvent::Blacklist, 1).await;
    }

    pub async fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let blacklist_arc = self.blacklist.clone();

        let mut return_list = vec![];
        for item in blacklist_arc.iter() { return_list.push(*item.key()); }

        return_list
    }

    pub async fn remove_flag_blacklist(&self, info_hash: InfoHash)
    {
        let blacklist_arc = self.blacklist.clone();

        if blacklist_arc.get(&info_hash).is_some() { blacklist_arc.insert(info_hash, 0i64); }

        let mut blacklist_count = 0i64;
        for item in blacklist_arc.iter() { if item.value() == &1i64 { blacklist_count += 1; } }

        self.set_stats(StatsEvent::Blacklist, blacklist_count).await;
    }

    pub async fn remove_blacklist(&self, info_hash: InfoHash)
    {
        let blacklist_arc = self.blacklist.clone();

        blacklist_arc.remove(&info_hash);

        let mut blacklist_count = 0i64;
        for item in blacklist_arc.iter() { if item.value() == &1 { blacklist_count += 1; } }

        self.set_stats(StatsEvent::Blacklist, blacklist_count).await;
    }

    pub async fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let blacklist_arc = self.blacklist.clone();

        if blacklist_arc.get(&info_hash).is_some() { return true; }

        false
    }

    pub async fn clear_blacklist(&self)
    {
        let blacklist_arc = self.blacklist.clone();

        blacklist_arc.clear();

        self.set_stats(StatsEvent::Blacklist, 0).await;
    }
}