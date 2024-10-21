use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::SystemTime;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub fn add_torrent_update(&self, info_hash: InfoHash, torrent_entry: TorrentEntry, updates_action: UpdatesAction) -> bool
    {
        let map = self.torrents_updates.clone();
        let mut lock = map.write();
        match lock.insert(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(), (info_hash, torrent_entry.clone(), updates_action)) {
            None => {
                self.update_stats(StatsEvent::TorrentsUpdates, 1);
                true
            }
            Some(_) => {
                false
            }
        }
    }

    pub fn add_torrent_updates(&self, hashes: HashMap<u128, (InfoHash, TorrentEntry, UpdatesAction)>) -> BTreeMap<InfoHash, bool>
    {
        let mut returned_data = BTreeMap::new();
        for (timestamp, (info_hash, torrent_entry, updates_action)) in hashes.iter() {
            returned_data.insert(*info_hash, self.add_torrent_update(*info_hash, torrent_entry.clone(), *updates_action));
            let _ = self.remove_torrent_update(timestamp);
        }
        returned_data
    }

    pub fn get_torrent_updates(&self) -> HashMap<u128, (InfoHash, TorrentEntry, UpdatesAction)>
    {
        let map = self.torrents_updates.clone();
        let lock = map.read_recursive();
        lock.clone()
    }

    pub fn remove_torrent_update(&self, timestamp: &u128) -> bool
    {
        let map = self.torrents_updates.clone();
        let mut lock = map.write();
        match lock.remove(timestamp) {
            None => { false }
            Some(_) => {
                self.update_stats(StatsEvent::TorrentsUpdates, -1);
                true
            }
        }
    }

    pub fn clear_torrent_updates(&self)
    {
        let map = self.torrents_updates.clone();
        let mut lock = map.write();
        lock.clear();
        self.set_stats(StatsEvent::TorrentsUpdates, 0);
    }

    pub async fn save_torrent_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let mut mapping: HashMap<InfoHash, (u128, TorrentEntry, UpdatesAction)> = HashMap::new();
        for (timestamp, (info_hash, torrent_entry, updates_action)) in self.get_torrent_updates().iter() {
            match mapping.entry(*info_hash) {
                Entry::Occupied(mut o) => {
                    o.insert((o.get().0, torrent_entry.clone(), *updates_action));
                    self.remove_torrent_update(timestamp);
                }
                Entry::Vacant(v) => {
                    v.insert((*timestamp, torrent_entry.clone(), *updates_action));
                }
            }
        }
        match self.save_torrents(torrent_tracker.clone(), mapping.clone().into_iter().map(|(info_hash, (_, torrent_entry, updates_action))| {
            (info_hash, (torrent_entry.clone(), updates_action))
        }).collect::<BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>>()).await {
            Ok(_) => {
                info!("[SYNC TORRENT UPDATES] Synced {} torrents", mapping.len());
                for (_, (timestamp, _, _)) in mapping.into_iter() {
                    self.remove_torrent_update(&timestamp);
                }
                Ok(())
            }
            Err(_) => {
                error!("[SYNC TORRENT UPDATES] Unable to sync {} torrents", mapping.len());
                Err(())
            }
        }
    }
}