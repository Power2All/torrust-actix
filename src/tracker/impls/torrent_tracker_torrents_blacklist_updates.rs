use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

impl TorrentTracker {
    pub fn add_blacklist_update(&self, info_hash: InfoHash, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.torrents_blacklist_updates.write();
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        if lock.insert(timestamp, (info_hash, updates_action)).is_none() {
            self.update_stats(StatsEvent::BlacklistUpdates, 1);
            true
        } else {
            false
        }
    }

    pub fn add_blacklist_updates(&self, hashes: Vec<(InfoHash, UpdatesAction)>) -> Vec<(InfoHash, bool)>
    {
        let mut lock = self.torrents_blacklist_updates.write();
        let mut returned_data = Vec::with_capacity(hashes.len());
        let mut success_count = 0i64;
        for (info_hash, updates_action) in hashes {
            let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
            let success = lock.insert(timestamp, (info_hash, updates_action)).is_none();
            if success {
                success_count += 1;
            }
            returned_data.push((info_hash, success));
        }
        if success_count > 0 {
            self.update_stats(StatsEvent::BlacklistUpdates, success_count);
        }
        returned_data
    }

    pub fn get_blacklist_updates(&self) -> HashMap<u128, (InfoHash, UpdatesAction)>
    {
        let lock = self.torrents_blacklist_updates.read_recursive();
        lock.clone()
    }

    pub fn remove_blacklist_update(&self, timestamp: &u128) -> bool
    {
        let mut lock = self.torrents_blacklist_updates.write();
        if lock.remove(timestamp).is_some() {
            self.update_stats(StatsEvent::BlacklistUpdates, -1);
            true
        } else {
            false
        }
    }

    pub fn clear_blacklist_updates(&self)
    {
        let mut lock = self.torrents_blacklist_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::BlacklistUpdates, 0);
    }

    pub async fn save_blacklist_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let updates = {
            let lock = self.torrents_blacklist_updates.read_recursive();
            lock.clone()
        };
        if updates.is_empty() {
            return Ok(());
        }
        let mut mapping: HashMap<InfoHash, (u128, UpdatesAction)> = HashMap::with_capacity(updates.len());
        let mut timestamps_to_remove = Vec::new();
        for (timestamp, (info_hash, updates_action)) in updates {
            match mapping.entry(info_hash) {
                Entry::Occupied(mut o) => {
                    let existing = o.get();
                    if timestamp > existing.0 {
                        timestamps_to_remove.push(existing.0);
                        o.insert((timestamp, updates_action));
                    } else {
                        timestamps_to_remove.push(timestamp);
                    }
                }
                Entry::Vacant(v) => {
                    v.insert((timestamp, updates_action));
                }
            }
        }
        let mapping_len = mapping.len();
        let blacklist_updates: Vec<(InfoHash, UpdatesAction)> = mapping
            .iter()
            .map(|(info_hash, (_, updates_action))| (*info_hash, *updates_action))
            .collect();
        match self.save_blacklist(torrent_tracker, blacklist_updates).await {
            Ok(_) => {
                info!("[SYNC BLACKLIST UPDATES] Synced {mapping_len} blacklists");
                let mut lock = self.torrents_blacklist_updates.write();
                let mut removed_count = 0i64;
                for (_, (timestamp, _)) in mapping {
                    if lock.remove(&timestamp).is_some() {
                        removed_count += 1;
                    }
                }
                for timestamp in timestamps_to_remove {
                    if lock.remove(&timestamp).is_some() {
                        removed_count += 1;
                    }
                }
                if removed_count > 0 {
                    self.update_stats(StatsEvent::BlacklistUpdates, -removed_count);
                }
                Ok(())
            }
            Err(_) => {
                error!("[SYNC BLACKLIST UPDATES] Unable to sync {mapping_len} blacklists");
                Err(())
            }
        }
    }
}