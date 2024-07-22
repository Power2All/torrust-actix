use std::collections::HashMap;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn add_torrents_update(&self, info_hash: InfoHash, completed: i64)
    {
        let updates_arc = self.torrents_updates.clone();

        updates_arc.insert(info_hash, completed);
        let update_count = updates_arc.len() as i64;

        self.set_stats(StatsEvent::TorrentsUpdates, update_count);
    }

    pub async fn add_torrents_updates(&self, updates: HashMap<InfoHash, i64>)
    {
        let updates_arc = self.torrents_updates.clone();

        let mut update_count = 0;

        for (info_hash, completed) in updates.iter() {
            updates_arc.insert(*info_hash, *completed);
            update_count = updates_arc.len();
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64);
    }

    pub async fn get_torrents_update(&self) -> HashMap<InfoHash, i64>
    {
        let updates_arc = self.torrents_updates.clone();

        let mut updates = HashMap::new();
        for item in updates_arc.iter() { updates.insert(*item.key(), *item.value()); }

        updates
    }

    pub async fn remove_torrents_update(&self, info_hash: InfoHash)
    {
        let updates_arc = self.torrents_updates.clone();

        updates_arc.remove(&info_hash);
        let update_count = updates_arc.len();

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64);
    }

    pub async fn remove_torrents_updates(&self, hashes: Vec<InfoHash>)
    {
        let updates_arc = self.torrents_updates.clone();

        let mut update_count = 0;

        for info_hash in hashes.iter() {
            updates_arc.remove(info_hash);
            update_count = updates_arc.len();
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64);
    }

    pub async fn transfer_torrents_updates_to_torrents_shadow(&self)
    {
        let updates_arc = self.torrents_updates.clone();

        for item in updates_arc.iter() {
            self.add_torrents_shadow(*item.key(), *item.value()).await;
            updates_arc.remove(item.key());
        }

        self.set_stats(StatsEvent::TorrentsUpdates, 0);
    }
}
