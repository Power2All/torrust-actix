use log::info;
use std::collections::HashMap;

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

impl TorrentTracker {
    pub async fn load_whitelists(&self)
    {
        if let Ok(whitelists) = self.sqlx.load_whitelist().await {
            let mut whitelist_count = 0i64;

            for info_hash in whitelists.iter() {
                self.add_whitelist(*info_hash, true).await;
                whitelist_count += 1;
            }

            info!("Loaded {} whitelists.", whitelist_count);
        }
    }

    pub async fn save_whitelists(&self) -> bool
    {
        let whitelist = self.get_whitelist().await;
        if self.sqlx.save_whitelist(whitelist.clone()).await.is_ok() {
            for (info_hash, value) in whitelist.iter() {
                if value == &0 {
                    self.remove_whitelist(*info_hash).await;
                }
                if value == &2 {
                    self.add_whitelist(*info_hash, true).await;
                }
            }
            return true;
        }
        false
    }

    pub async fn add_whitelist(&self, info_hash: InfoHash, on_load: bool)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        if on_load {
            whitelist_lock.insert(info_hash, 1i64);
        } else {
            whitelist_lock.insert(info_hash, 2i64);
        }
        drop(whitelist_lock);

        self.update_stats(StatsEvent::Whitelist, 1).await;
    }

    pub async fn get_whitelist(&self) -> HashMap<InfoHash, i64>
    {
        let mut return_list = HashMap::new();

        let whitelist_arc = self.whitelist.clone();
        let whitelist_lock = whitelist_arc.read().await;
        for (info_hash, value) in whitelist_lock.iter() {
            return_list.insert(*info_hash, *value);
        }
        drop(whitelist_lock);

        return_list
    }

    pub async fn remove_flag_whitelist(&self, info_hash: InfoHash)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        if whitelist_lock.get(&info_hash).is_some() {
            whitelist_lock.insert(info_hash, 0i64);
        }
        let whitelists = whitelist_lock.clone();
        drop(whitelist_lock);

        let mut whitelist_count = 0i64;
        for (_, value) in whitelists.iter() {
            if value == &1i64 {
                whitelist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn remove_whitelist(&self, info_hash: InfoHash)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        whitelist_lock.remove(&info_hash);
        let whitelists = whitelist_lock.clone();
        drop(whitelist_lock);

        let mut whitelist_count = 0i64;
        for (_, value) in whitelists.iter() {
            if value == &1 {
                whitelist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let whitelist_arc = self.whitelist.clone();
        let whitelist_lock = whitelist_arc.read().await;
        let whitelist = whitelist_lock.get(&info_hash).cloned();
        drop(whitelist_lock);

        if whitelist.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_whitelist(&self)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        whitelist_lock.clear();
        drop(whitelist_lock);

        self.set_stats(StatsEvent::Whitelist, 0).await;
    }
}