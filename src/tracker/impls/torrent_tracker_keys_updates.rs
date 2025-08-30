use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::SystemTime;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub fn add_key_update(&self, info_hash: InfoHash, timeout: i64, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.keys_updates.write();
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();

        if lock.insert(timestamp, (info_hash, timeout, updates_action)).is_none() {
            self.update_stats(StatsEvent::KeyUpdates, 1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_key_updates(&self) -> HashMap<u128, (InfoHash, i64, UpdatesAction)>
    {
        let lock = self.keys_updates.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_key_update(&self, timestamp: &u128) -> bool
    {
        let mut lock = self.keys_updates.write();
        if lock.remove(timestamp).is_some() {
            self.update_stats(StatsEvent::KeyUpdates, -1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_key_updates(&self)
    {
        let mut lock = self.keys_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::KeyUpdates, 0);
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_key_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let updates = self.get_key_updates();

        let mut mapping: HashMap<InfoHash, (u128, i64, UpdatesAction)> = HashMap::new();
        let mut timestamps_to_remove = Vec::new();

        for (timestamp, (info_hash, timeout, updates_action)) in updates {
            match mapping.entry(info_hash) {
                Entry::Occupied(mut o) => {
                    let existing = o.get();
                    if timestamp > existing.0 {
                        timestamps_to_remove.push(existing.0);
                        o.insert((timestamp, timeout, updates_action));
                    } else {
                        timestamps_to_remove.push(timestamp);
                    }
                }
                Entry::Vacant(v) => {
                    v.insert((timestamp, timeout, updates_action));
                }
            }
        }

        let keys_to_save: BTreeMap<InfoHash, (i64, UpdatesAction)> = mapping
            .iter()
            .map(|(info_hash, (_, timeout, updates_action))| (*info_hash, (*timeout, *updates_action)))
            .collect();

        match self.save_keys(torrent_tracker, keys_to_save).await {
            Ok(_) => {
                info!("[SYNC KEY UPDATES] Synced {} keys", mapping.len());

                for (_, (timestamp, _, _)) in mapping {
                    self.remove_key_update(&timestamp);
                }

                for timestamp in timestamps_to_remove {
                    self.remove_key_update(&timestamp);
                }

                Ok(())
            }
            Err(_) => {
                error!("[SYNC KEY UPDATES] Unable to sync {} keys", mapping.len());
                Err(())
            }
        }
    }
}