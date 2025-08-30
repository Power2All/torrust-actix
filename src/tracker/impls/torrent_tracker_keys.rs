use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(keys) = self.sqlx.load_keys(tracker).await {
            info!("Loaded {keys} keys");
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<(), ()>
    {
        match self.sqlx.save_keys(tracker, keys).await {
            Ok(keys_count) => {
                info!("[SYNC KEYS] Synced {keys_count} keys");
                Ok(())
            }
            Err(_) => {
                error!("[SYNC KEYS] Unable to sync keys");
                Err(())
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_key(&self, hash: InfoHash, timeout: i64) -> bool
    {
        let mut lock = self.keys.write();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_unix = timestamp.as_secs() as i64 + timeout;

        match lock.entry(hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Key, 1);
                v.insert(timeout_unix);
                true
            }
            Entry::Occupied(mut o) => {
                o.insert(timeout_unix);
                false
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_key(&self, hash: InfoHash) -> Option<(InfoHash, i64)>
    {
        let lock = self.keys.read_recursive();
        lock.get(&hash).map(|&data| (hash, data))
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_keys(&self) -> BTreeMap<InfoHash, i64>
    {
        let lock = self.keys.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_key(&self, hash: InfoHash) -> bool
    {
        let mut lock = self.keys.write();
        if lock.remove(&hash).is_some() {
            self.update_stats(StatsEvent::Key, -1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn check_key(&self, hash: InfoHash) -> bool
    {
        let lock = self.keys.read_recursive();
        lock.get(&hash).map_or(false, |&key| {
            let key_time = Utc.timestamp_opt(key, 0)
                .single()
                .map(|dt| SystemTime::from(dt))
                .unwrap_or(UNIX_EPOCH);

            key_time > SystemTime::now()
        })
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_keys(&self)
    {
        let mut lock = self.keys.write();
        lock.clear();
        self.set_stats(StatsEvent::Key, 0);
    }

    #[tracing::instrument(level = "debug")]
    pub fn clean_keys(&self)
    {
        let now = SystemTime::now();
        let mut keys_to_remove = Vec::new();

        {
            let lock = self.keys.read_recursive();
            for (&hash, &key_time) in lock.iter() {
                let time = Utc.timestamp_opt(key_time, 0)
                    .single()
                    .map(|dt| SystemTime::from(dt))
                    .unwrap_or(UNIX_EPOCH);

                if time <= now {
                    keys_to_remove.push(hash);
                }
            }
        }

        for hash in keys_to_remove {
            self.remove_key(hash);
        }
    }
}