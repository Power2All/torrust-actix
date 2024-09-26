use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::SystemTime;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub fn add_torrent_update(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) -> (TorrentEntry, bool)
    {
        let map = self.torrents_updates.clone();
        let mut lock = map.write();
        match lock.insert(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(), (info_hash, torrent_entry.clone())) {
            None => {
                self.update_stats(StatsEvent::TorrentsUpdates, 1);
                (torrent_entry, true)
            }
            Some(_) => {
                (torrent_entry, false)
            }
        }
    }

    pub fn add_torrent_updates(&self, hashes: HashMap<u128, (InfoHash, TorrentEntry)>) -> BTreeMap<InfoHash, (TorrentEntry, bool)>
    {
        let mut returned_data = BTreeMap::new();
        for (timestamp, (info_hash, torrent_entry)) in hashes.iter() {
            returned_data.insert(*info_hash, self.add_torrent_update(*info_hash, torrent_entry.clone()));
            let _ = self.remove_torrent_update(timestamp);
        }
        returned_data
    }

    pub fn get_torrent_updates(&self) -> HashMap<u128, (InfoHash, TorrentEntry)>
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
        let mut hashmapping: HashMap<InfoHash, (Vec<u128>, TorrentEntry)> = HashMap::new();
        let mut hashmap: BTreeMap<InfoHash, TorrentEntry> = BTreeMap::new();
        let updates = self.get_torrent_updates();

        // Build the actually updates for SQL, adding the timestamps into a vector for removal afterward.
        for (timestamp, (info_hash, torrent_entry)) in updates.iter() {
            match hashmapping.get_mut(info_hash) {
                None => {
                    hashmapping.insert(*info_hash, (vec![*timestamp], torrent_entry.clone()));
                    hashmap.insert(*info_hash, torrent_entry.clone());
                }
                Some((timestamps, _)) => {
                    if !timestamps.contains(timestamp) {
                        timestamps.push(*timestamp);
                    }
                    hashmap.insert(*info_hash, torrent_entry.clone());
                }
            }
        }

        // Now we're going to save the torrents in a list, and depending on what we get returned, we remove them from the updates list.
        match self.save_torrents(torrent_tracker.clone(), hashmap).await {
            Ok(_) => {
                // We can remove the updates keys, since they are updated.
                for (_, (timestamps, _)) in hashmapping.iter() {
                    for timestamp in timestamps.iter() {
                        self.remove_torrent_update(timestamp);
                    }
                }
                info!("[SAVE TORRENTS UPDATES] Saved {} torrents", hashmapping.len());
                Ok(())
            }
            Err(_) => {
                error!("[SAVE TORRENTS UPDATES] Unable to save {} torrents", hashmapping.len());
                Err(())
            }
        }
    }
}