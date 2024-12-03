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
        let map = self.keys_updates.clone();
        let mut lock = map.write();
        match lock.insert(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(), (info_hash, timeout, updates_action)) {
            None => {
                self.update_stats(StatsEvent::KeyUpdates, 1);
                true
            }
            Some(_) => {
                false
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_key_updates(&self) -> HashMap<u128, (InfoHash, i64, UpdatesAction)>
    {
        let map = self.keys_updates.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_key_update(&self, timestamp: &u128) -> bool
    {
        let map = self.keys_updates.clone();
        let mut lock = map.write();
        match lock.remove(timestamp) {
            None => { false }
            Some(_) => {
                self.update_stats(StatsEvent::KeyUpdates, -1);
                true
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_key_updates(&self)
    {
        let map = self.keys_updates.clone();
        let mut lock = map.write();
        lock.clear();
        self.set_stats(StatsEvent::KeyUpdates, 0);
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_key_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let mut mapping: HashMap<InfoHash, (u128, i64, UpdatesAction)> = HashMap::new();
        for (timestamp, (info_hash, timeout, updates_action)) in self.get_key_updates().iter() {
            match mapping.entry(*info_hash) {
                Entry::Occupied(mut o) => {
                    o.insert((o.get().0, *timeout, *updates_action));
                    self.remove_key_update(timestamp);
                }
                Entry::Vacant(v) => {
                    v.insert((*timestamp, *timeout, *updates_action));
                }
            }
        }
        match self.save_keys(torrent_tracker.clone(), mapping.clone().into_iter().map(|(info_hash, (_, timeout, updates_action))| {
            (info_hash, (timeout, updates_action))
        }).collect::<BTreeMap<InfoHash, (i64, UpdatesAction)>>()).await {
            Ok(_) => {
                info!("[SYNC KEY UPDATES] Synced {} keys", mapping.len());
                for (_, (timestamp, _, _)) in mapping.into_iter() {
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