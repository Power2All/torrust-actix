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
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(keys) = self.sqlx.load_keys(tracker.clone()).await {
            info!("Loaded {} keys", keys);
        }
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<(), ()>
    {
        match self.sqlx.save_keys(tracker.clone(), keys.clone()).await {
            Ok(keys_count) => {
                info!("[SYNC KEYS] Synced {} keys", keys_count);
                Ok(())
            }
            Err(_) => {
                error!("[SYNC KEYS] Unable to sync {} keys", keys.len());
                Err(())
            }
        }
    }

    pub fn add_key(&self, hash: InfoHash, timeout: i64) -> bool
    {
        let map = self.keys.clone();
        let mut lock = map.write();
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

    pub fn get_key(&self, hash: InfoHash) -> Option<(InfoHash, i64)>
    {
        let map = self.keys.clone();
        let lock = map.read_recursive();
        lock.get(&hash).map(|data| (hash, *data))
    }

    pub fn get_keys(&self) -> BTreeMap<InfoHash, i64>
    {
        let map = self.keys.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    pub fn remove_key(&self, hash: InfoHash) -> bool
    {
        let map = self.keys.clone();
        let mut lock = map.write();
        match lock.remove(&hash) {
            None => {
                false
            }
            Some(_) => {
                self.update_stats(StatsEvent::Key, -1);
                true
            }
        }
    }

    pub fn check_key(&self, hash: InfoHash) -> bool
    {
        let map = self.keys.clone();
        let lock = map.read_recursive();
        match lock.get(&hash) {
            None => {
                false
            }
            Some(key) => {
                let time = SystemTime::from(Utc.timestamp_opt(*key, 0).unwrap());
                match time.duration_since(SystemTime::now()) {
                    Ok(_) => {
                        true
                    }
                    Err(_) => {
                        false
                    }
                }
            }
        }
    }

    pub fn clear_keys(&self)
    {
        let map = self.keys.clone();
        let mut lock = map.write();
        lock.clear();
        self.set_stats(StatsEvent::Key, 0);
    }

    pub fn clean_keys(&self)
    {
        let keys = self.get_keys();
        for (hash, key_time) in keys.iter() {
            let time = SystemTime::from(Utc.timestamp_opt(*key_time, 0).unwrap());
            if time.duration_since(SystemTime::now()).is_err() {
                self.remove_key(*hash);
            }
        }
    }
}