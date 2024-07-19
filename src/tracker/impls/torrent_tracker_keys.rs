use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use log::info;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(keys) = self.sqlx.load_keys(tracker.clone()).await {
            let mut keys_count = 0i64;

            for (hash, timeout) in keys.iter() {
                self.add_key_raw(*hash, *timeout).await;
                keys_count += 1;
            }

            info!("Loaded {} keys.", keys_count);
        }
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>) -> bool
    {
        let keys = self.get_keys().await;

        if self.sqlx.save_keys(tracker.clone(), keys).await.is_ok() { return true; }

        false
    }

    pub async fn add_key(&self, hash: InfoHash, timeout: i64)
    {
        let keys_arc = self.keys.clone();

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_unix = timestamp.as_secs() as i64 + timeout;
        keys_arc.insert(hash, timeout_unix);

        self.update_stats(StatsEvent::Key, 1);
    }

    pub async fn add_key_raw(&self, hash: InfoHash, timeout: i64)
    {
        let keys_arc = self.keys.clone();

        let time = SystemTime::from(Utc.timestamp_opt(timeout, 0).unwrap());
        if time.duration_since(SystemTime::now()).is_ok() { keys_arc.insert(hash, timeout); } else { return; }

        self.update_stats(StatsEvent::Key, 1);
    }

    pub async fn get_keys(&self) -> Vec<(InfoHash, i64)>
    {
        let keys_arc = self.keys.clone();

        let mut return_list = vec![];
        for item in keys_arc.iter() { return_list.push((*item.key(), *item.value())); }

        return_list
    }

    pub async fn remove_key(&self, hash: InfoHash)
    {
        let keys_arc = self.keys.clone();

        keys_arc.remove(&hash);
        let key_count = keys_arc.len();

        self.set_stats(StatsEvent::Key, key_count as i64);
    }

    pub async fn check_key(&self, hash: InfoHash) -> bool
    {
        let keys_arc = self.keys.clone();

        if keys_arc.get(&hash).is_some() { return true; }

        false
    }

    pub async fn clear_keys(&self)
    {
        let keys_arc = self.keys.clone();

        keys_arc.clear();

        self.set_stats(StatsEvent::Key, 0);
    }

    pub async fn clean_keys(&self)
    {
        let keys_arc = self.keys.clone();

        let mut keys_index = vec![];
        for item in keys_arc.iter() { keys_index.push((*item.key(), *item.value())); }

        for (hash, timeout) in keys_index.iter() {
            if *timeout != 0 {
                let time = SystemTime::from(Utc.timestamp_opt(*timeout, 0).unwrap());
                if time.duration_since(SystemTime::now()).is_err() { self.remove_key(*hash).await; }
            }
        }
    }
}
