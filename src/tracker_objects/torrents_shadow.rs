use scc::ebr::Arc;
use std::collections::HashMap;

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

impl TorrentTracker {
    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<bool, ()>
    {
        if self.sqlx.save_torrents(tracker.clone(), self.get_torrents_shadow().await).await.is_ok() {
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn add_torrents_shadow(&self, info_hash: InfoHash, completed: i64)
    {
        let shadow_arc = self.torrents_shadow.clone();

        shadow_arc.insert(info_hash, completed);
        let shadow_count = shadow_arc.len();

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_torrents_shadow(&self, info_hash: InfoHash)
    {
        let shadow_arc = self.torrents_shadow.clone();

        shadow_arc.remove(&info_hash);
        let shadow_count = shadow_arc.len();

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_torrents_shadows(&self, hashes: Vec<InfoHash>)
    {
        let shadow_arc = self.torrents_shadow.clone();

        let mut shadow_count = 0;
        for info_hash in hashes.iter() {
            shadow_arc.remove(info_hash);
            shadow_count = shadow_arc.len();
        }

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn get_torrents_shadow(&self) -> HashMap<InfoHash, i64>
    {
        let shadow_arc = self.torrents_shadow.clone();

        let mut shadow = HashMap::new();
        for item in shadow_arc.iter() { shadow.insert(*item.key(), *item.value()); }

        shadow
    }

    pub async fn clear_torrents_shadow(&self)
    {
        let shadow_arc = self.torrents_shadow.clone();

        shadow_arc.clear();

        self.set_stats(StatsEvent::TorrentsShadow, 0).await;
    }
}