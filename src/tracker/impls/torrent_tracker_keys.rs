use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use chrono::{
    TimeZone,
    Utc
};
use log::{
    error,
    info
};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

impl TorrentTracker {
    /// Loads all announce keys from the configured database into memory at startup.
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok(keys) = self.sqlx.load_keys(tracker).await {
            info!("Loaded {keys} keys");
        }
    }

    /// Persists the given batch of announce keys (and their add/remove actions) to the database.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database write fails; the caller re-queues the batch.
    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<(), ()>
    {
        if let Ok(keys_count) = self.sqlx.save_keys(tracker, keys).await {
            info!("[SYNC KEYS] Synced {keys_count} keys");
            Ok(())
        } else {
            error!("[SYNC KEYS] Unable to sync keys");
            Err(())
        }
    }

    /// Adds an announce key expiring `timeout` seconds from now.
    ///
    /// Use [`TorrentTracker::add_key_absolute`] when the expiry is already an absolute
    /// timestamp, such as a value read back from the database.
    ///
    /// Returns `true` when the key was newly inserted, `false` when it was refreshed.
    pub fn add_key(&self, hash: InfoHash, timeout: i64) -> bool
    {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.add_key_absolute(hash, timestamp.as_secs() as i64 + timeout)
    }

    /// Adds an announce key with an absolute expiry timestamp, stored verbatim.
    ///
    /// An expiry of `0` means the key never expires; any other value is a Unix timestamp,
    /// including ones already in the past.
    ///
    /// Returns `true` when the key was newly inserted, `false` when it was refreshed.
    pub fn add_key_absolute(&self, hash: InfoHash, expiry: i64) -> bool
    {
        let mut lock = self.keys.write();
        match lock.entry(hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Key, 1);
                v.insert(expiry);
                true
            }
            Entry::Occupied(mut o) => {
                o.insert(expiry);
                false
            }
        }
    }

    /// Returns the key and its expiry timestamp when the key exists.
    pub fn get_key(&self, hash: InfoHash) -> Option<(InfoHash, i64)>
    {
        let lock = self.keys.read_recursive();
        lock.get(&hash).map(|&data| (hash, data))
    }

    /// Returns a clone of the complete key table.
    pub fn get_keys(&self) -> BTreeMap<InfoHash, i64>
    {
        let lock = self.keys.read_recursive();
        lock.clone()
    }

    /// Removes an announce key; returns `true` when it existed.
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

    /// Returns `true` when the announce key exists and has not expired (used on every keyed
    /// announce/scrape). An expiry of `0` means the key never expires.
    pub fn check_key(&self, hash: InfoHash) -> bool
    {
        let lock = self.keys.read_recursive();
        lock.get(&hash).is_some_and(|&key| {
            if key == 0 {
                return true;
            }
            let key_time = Utc.timestamp_opt(key, 0)
                .single()
                .map_or(UNIX_EPOCH, SystemTime::from);
            key_time > SystemTime::now()
        })
    }

    /// Removes all announce keys and resets the key counter statistic.
    pub fn clear_keys(&self)
    {
        let mut lock = self.keys.write();
        lock.clear();
        self.set_stats(StatsEvent::Key, 0);
    }

    /// Removes every announce key whose expiry timestamp has passed.
    ///
    /// Keys with an expiry of `0` are permanent and are left alone.
    ///
    /// Runs periodically from the key-cleanup task.
    pub fn clean_keys(&self)
    {
        let now = SystemTime::now();
        let mut keys_to_remove = Vec::new();
        {
            let lock = self.keys.read_recursive();
            for (&hash, &key_time) in lock.iter() {
                if key_time == 0 {
                    continue;
                }
                let time = Utc.timestamp_opt(key_time, 0)
                    .single()
                    .map_or(UNIX_EPOCH, SystemTime::from);
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