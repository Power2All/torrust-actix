use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::info;
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

    /// Converts a relative `timeout` in seconds into the absolute expiry stored for a key:
    /// `0` stays `0` (never expires), anything else is that far from now, saturating rather
    /// than wrapping.
    ///
    /// The in-memory entry and the queued database update must both carry this value: the
    /// database load reads it back as absolute through [`TorrentTracker::add_key_absolute`].
    pub fn key_expiry(timeout: u64) -> i64
    {
        if timeout == 0 {
            return 0;
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        i64::try_from(now.saturating_add(timeout)).unwrap_or(i64::MAX)
    }

    /// Adds an announce key expiring `timeout` seconds from now (`0` = never).
    ///
    /// Use [`TorrentTracker::add_key_absolute`] when the expiry is already an absolute
    /// timestamp, such as a value read back from the database.
    ///
    /// Returns `true` when the key was newly inserted, `false` when it was refreshed.
    pub fn add_key(&self, hash: InfoHash, timeout: u64) -> bool
    {
        self.add_key_absolute(hash, Self::key_expiry(timeout))
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
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        lock.get(&hash).is_some_and(|&key| key == 0 || u64::try_from(key).is_ok_and(|key| key > now))
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
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut keys_to_remove = Vec::new();
        {
            let lock = self.keys.read_recursive();
            for (&hash, &key_time) in lock.iter() {
                if key_time == 0 {
                    continue;
                }
                if !u64::try_from(key_time).is_ok_and(|key_time| key_time > now) {
                    keys_to_remove.push(hash);
                }
            }
        }
        for hash in keys_to_remove {
            self.remove_key(hash);
        }
    }
}