use std::collections::HashMap;

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

impl TorrentTracker {
    pub async fn add_update(&self, info_hash: InfoHash, completed: i64)
    {
        let updates_arc = self.torrents_updates.clone();

        updates_arc.insert(info_hash, completed);
        let update_count = updates_arc.len() as i64;

        self.set_stats(StatsEvent::TorrentsUpdates, update_count).await;
    }

    pub async fn add_updates(&self, updates: HashMap<InfoHash, i64>)
    {
        let updates_arc = self.torrents_updates.clone();

        let mut update_count = 0;

        for (info_hash, completed) in updates.iter() {
            updates_arc.insert(*info_hash, *completed);
            update_count = updates_arc.len();
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn get_update(&self) -> HashMap<InfoHash, i64>
    {
        let updates_arc = self.torrents_updates.clone();

        let mut updates = HashMap::new();
        for item in updates_arc.iter() { updates.insert(*item.key(), *item.value()); }

        updates
    }

    pub async fn remove_update(&self, info_hash: InfoHash)
    {
        let updates_arc = self.torrents_updates.clone();

        updates_arc.remove(&info_hash);
        let update_count = updates_arc.len();

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn remove_updates(&self, hashes: Vec<InfoHash>)
    {
        let updates_arc = self.torrents_updates.clone();

        let mut update_count = 0;

        for info_hash in hashes.iter() {
            updates_arc.remove(info_hash);
            update_count = updates_arc.len();
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn transfer_updates_to_shadow(&self)
    {
        let updates_arc = self.torrents_updates.clone();

        for item in updates_arc.iter() {
            self.add_shadow(*item.key(), *item.value()).await;
            updates_arc.remove(item.key());
        }

        self.set_stats(StatsEvent::TorrentsUpdates, 0).await;
    }
}