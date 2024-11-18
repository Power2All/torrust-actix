use std::collections::HashMap;
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
    pub fn add_whitelist_update(&self, info_hash: InfoHash, updates_action: UpdatesAction) -> bool
    {
        let map = self.torrents_whitelist_updates.clone();
        let mut lock = map.write();
        match lock.insert(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(), (info_hash, updates_action)) {
            None => {
                self.update_stats(StatsEvent::WhitelistUpdates, 1);
                true
            }
            Some(_) => {
                false
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_whitelist_updates(&self, hashes: Vec<(InfoHash, UpdatesAction)>) -> Vec<(InfoHash, bool)>
    {
        let mut returned_data = Vec::new();
        for (info_hash, updates_action) in hashes {
            returned_data.push((info_hash, self.add_whitelist_update(info_hash, updates_action)));
        }
        returned_data
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_whitelist_updates(&self) -> HashMap<u128, (InfoHash, UpdatesAction)>
    {
        let map = self.torrents_whitelist_updates.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_whitelist_update(&self, timestamp: &u128) -> bool
    {
        let map = self.torrents_whitelist_updates.clone();
        let mut lock = map.write();
        match lock.remove(timestamp) {
            None => { false }
            Some(_) => {
                self.update_stats(StatsEvent::WhitelistUpdates, -1);
                true
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_whitelist_updates(&self)
    {
        let map = self.torrents_whitelist_updates.clone();
        let mut lock = map.write();
        lock.clear();
        self.set_stats(StatsEvent::WhitelistUpdates, 0);
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_whitelist_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let mut mapping: HashMap<InfoHash, (u128, UpdatesAction)> = HashMap::new();
        for (timestamp, (info_hash, updates_action)) in self.get_whitelist_updates().iter() {
            match mapping.entry(*info_hash) {
                Entry::Occupied(mut o) => {
                    o.insert((o.get().0, *updates_action));
                    self.remove_whitelist_update(timestamp);
                }
                Entry::Vacant(v) => {
                    v.insert((*timestamp, *updates_action));
                }
            }
        }
        match self.save_whitelist(torrent_tracker.clone(), mapping.clone().into_iter().map(|(info_hash, (_, updates_action))| {
            (info_hash, updates_action)
        }).collect::<Vec<(InfoHash, UpdatesAction)>>()).await {
            Ok(_) => {
                info!("[SYNC WHITELIST UPDATES] Synced {} whitelists", mapping.len());
                for (_, (timestamp, _)) in mapping.into_iter() {
                    self.remove_whitelist_update(&timestamp);
                }
                Ok(())
            }
            Err(_) => {
                error!("[SYNC WHITELIST UPDATES] Unable to sync {} whitelists", mapping.len());
                Err(())
            }
        }
    }
}