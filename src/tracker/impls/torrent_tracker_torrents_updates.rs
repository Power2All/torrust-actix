use crate::cache::traits::cache_backend::CacheBackend;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    debug,
    error,
    info,
    warn
};
use std::collections::hash_map::Entry;
use std::collections::{
    BTreeMap,
    HashMap
};
use std::sync::Arc;
use std::time::SystemTime;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub fn add_torrent_update(&self, info_hash: InfoHash, torrent_entry: TorrentEntry, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.torrents_updates.write();
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        if lock.insert(timestamp, (info_hash, torrent_entry, updates_action)).is_none() {
            self.update_stats(StatsEvent::TorrentsUpdates, 1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn add_torrent_updates(&self, hashes: HashMap<u128, (InfoHash, TorrentEntry, UpdatesAction)>) -> BTreeMap<InfoHash, bool>
    {
        let mut lock = self.torrents_updates.write();
        let mut returned_data = BTreeMap::new();
        let mut success_count = 0i64;
        let mut remove_count = 0i64;
        for (timestamp, (info_hash, torrent_entry, updates_action)) in hashes {
            let new_timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
            let success = lock.insert(new_timestamp, (info_hash, torrent_entry, updates_action)).is_none();
            if success {
                success_count += 1;
            }
            returned_data.insert(info_hash, success);
            if lock.remove(&timestamp).is_some() {
                remove_count += 1;
            }
        }
        if success_count > 0 {
            self.update_stats(StatsEvent::TorrentsUpdates, success_count);
        }
        if remove_count > 0 {
            self.update_stats(StatsEvent::TorrentsUpdates, -remove_count);
        }
        returned_data
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_torrent_updates(&self) -> HashMap<u128, (InfoHash, TorrentEntry, UpdatesAction)>
    {
        let lock = self.torrents_updates.read_recursive();
        lock.clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn remove_torrent_update(&self, timestamp: &u128) -> bool
    {
        let mut lock = self.torrents_updates.write();
        if lock.remove(timestamp).is_some() {
            self.update_stats(StatsEvent::TorrentsUpdates, -1);
            true
        } else {
            false
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn clear_torrent_updates(&self)
    {
        let mut lock = self.torrents_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::TorrentsUpdates, 0);
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_torrent_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let updates = {
            let lock = self.torrents_updates.read_recursive();
            lock.clone()
        };
        if updates.is_empty() {
            return Ok(());
        }
        let mut mapping: HashMap<InfoHash, (u128, TorrentEntry, UpdatesAction)> = HashMap::with_capacity(updates.len());
        let mut timestamps_to_remove = Vec::new();
        for (timestamp, (info_hash, torrent_entry, updates_action)) in updates {
            match mapping.entry(info_hash) {
                Entry::Occupied(mut o) => {
                    let existing = o.get();
                    if timestamp > existing.0 {
                        timestamps_to_remove.push(existing.0);
                        o.insert((timestamp, torrent_entry, updates_action));
                    } else {
                        timestamps_to_remove.push(timestamp);
                    }
                }
                Entry::Vacant(v) => {
                    v.insert((timestamp, torrent_entry, updates_action));
                }
            }
        }
        let mapping_len = mapping.len();
        let torrents_to_save: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)> = mapping
            .iter()
            .map(|(info_hash, (_, torrent_entry, updates_action))| (*info_hash, (torrent_entry.clone(), *updates_action)))
            .collect();
        match self.save_torrents(torrent_tracker.clone(), torrents_to_save.clone()).await {
            Ok(_) => {
                info!("[SYNC TORRENT UPDATES] Synced {mapping_len} torrents");
                if let Some(ref cache) = self.cache {
                    let cache_ttl = self.config.cache.as_ref().and_then(|c| {
                        if c.ttl > 0 { Some(c.ttl) } else { None }
                    });
                    let cache_data: Vec<_> = torrents_to_save
                        .iter()
                        .filter(|(_, (_, action))| *action != UpdatesAction::Remove)
                        .map(|(hash, (entry, _))| (*hash, entry.seeds.len() as u64, entry.peers.len() as u64))
                        .collect();
                    if !cache_data.is_empty() {
                        match cache.set_torrent_peers_batch(&cache_data, cache_ttl).await {
                            Ok(_) => {
                                debug!("[Cache] Updated {} torrent peer counts", cache_data.len());
                            }
                            Err(e) => {
                                warn!("[Cache] Failed to update peer counts: {}", e);
                            }
                        }
                    }
                    for (hash, (_, action)) in &torrents_to_save {
                        if *action == UpdatesAction::Remove
                            && let Err(e) = cache.delete_torrent(hash).await {
                                warn!("[Cache] Failed to delete torrent {}: {}", hash, e);
                            }
                    }
                }
                let mut lock = self.torrents_updates.write();
                let mut removed_count = 0i64;
                for (_, (timestamp, _, _)) in mapping {
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
                    self.update_stats(StatsEvent::TorrentsUpdates, -removed_count);
                }
                Ok(())
            }
            Err(_) => {
                error!("[SYNC TORRENT UPDATES] Unable to sync {mapping_len} torrents");
                Err(())
            }
        }
    }
}