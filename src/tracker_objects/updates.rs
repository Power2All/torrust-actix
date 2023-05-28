use async_std::future::timeout;
use log::error;
use std::collections::HashMap;
use std::time::Duration;

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

impl TorrentTracker {
    pub async fn add_update(&self, info_hash: InfoHash, completed: i64)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.lock().await;
        updates_lock.insert(info_hash, completed);
        let update_count = updates_lock.len();
        drop(updates_lock);

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn add_updates(&self, updates: HashMap<InfoHash, i64>)
    {
        let mut update_count = 0;

        for (info_hash, completed) in updates.iter() {
            let updates_arc = self.updates.clone();
            let mut updates_lock = updates_arc.lock().await;
            updates_lock.insert(*info_hash, *completed);
            update_count = updates_lock.len();
            drop(updates_lock);
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn get_update(&self) -> Result<HashMap<InfoHash, i64>, ()>
    {
        let updates = match timeout(Duration::from_secs(30), async move {
            let updates_arc = self.updates.clone();
            let updates_lock = updates_arc.lock().await;
            let updates = updates_lock.clone();
            drop(updates_lock);
            updates
        }).await {
            Ok(data) => { data }
            Err(_) => {
                error!("[GET_UPDATE] Read Lock (updates) request timed out!");
                return Err(());
            }
        };

        Ok(updates)
    }

    pub async fn remove_update(&self, info_hash: InfoHash)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.lock().await;
        updates_lock.remove(&info_hash);
        let update_count = updates_lock.len();
        drop(updates_lock);

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn remove_updates(&self, hashes: Vec<InfoHash>)
    {
        let mut update_count = 0;

        for info_hash in hashes.iter() {
            let updates_arc = self.updates.clone();
            let mut updates_lock = updates_arc.lock().await;
            updates_lock.remove(info_hash);
            update_count = updates_lock.len();
            drop(updates_lock);
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn transfer_updates_to_shadow(&self)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.lock().await;
        let updates = updates_lock.clone();
        updates_lock.clear();
        drop(updates_lock);

        for (info_hash, completed) in updates.iter() {
            self.add_shadow(*info_hash, *completed).await;
        }

        self.set_stats(StatsEvent::TorrentsUpdates, 0).await;
    }
}