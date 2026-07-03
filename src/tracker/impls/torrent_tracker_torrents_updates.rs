use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;
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
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic, process-local sequence number used as the dedupe ordering
/// key for queued torrent updates.  Replaces wall-clock nanoseconds
/// (`SystemTime::now()`), which can jump backwards on NTP corrections /
/// leap seconds and would let an older update win the dedupe.
static UPDATE_SEQ: AtomicU64 = AtomicU64::new(0);

#[inline]
fn next_seq() -> u128 {
    u128::from(UPDATE_SEQ.fetch_add(1, Ordering::Relaxed))
}

impl TorrentTracker {
    /// Queues a torrent update for the next database/cache flush.
    ///
    /// Returns `true` when a new queue slot was created.
    pub fn add_torrent_update(&self, info_hash: InfoHash, torrent_update_data: TorrentUpdateData, updates_action: UpdatesAction) -> bool
    {
        let mut lock = self.torrents_updates.write();
        let timestamp = next_seq();
        if lock.insert(timestamp, (info_hash, torrent_update_data, updates_action)).is_none() {
            self.update_stats(StatsEvent::TorrentsUpdates, 1);
            true
        } else {
            false
        }
    }

    /// Re-queues a batch of torrent updates under fresh sequence numbers, removing the old slots.
    ///
    /// Returns, per info-hash, whether the insert created a new slot.
    pub fn add_torrent_updates(&self, hashes: HashMap<u128, (InfoHash, TorrentUpdateData, UpdatesAction)>) -> BTreeMap<InfoHash, bool>
    {
        let mut lock = self.torrents_updates.write();
        let mut returned_data = BTreeMap::new();
        let mut success_count = 0i64;
        let mut remove_count = 0i64;
        for (timestamp, (info_hash, torrent_entry, updates_action)) in hashes {
            let new_timestamp = next_seq();
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

    /// Returns a clone of the pending torrent-update queue.
    pub fn get_torrent_updates(&self) -> HashMap<u128, (InfoHash, TorrentUpdateData, UpdatesAction)>
    {
        let lock = self.torrents_updates.read_recursive();
        lock.clone()
    }

    /// Removes a single queued update by its sequence key; returns `true` when it existed.
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

    /// Drops all queued torrent updates and resets the queue statistic.
    pub fn clear_torrent_updates(&self)
    {
        let mut lock = self.torrents_updates.write();
        lock.clear();
        self.set_stats(StatsEvent::TorrentsUpdates, 0);
    }

    /// Drains the update queue, deduplicates it per info-hash (newest wins) and flushes the
    /// result to the database and/or peer-count cache.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database flush fails; the drained updates are restored to the
    /// queue so no data is lost.
    pub async fn save_torrent_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
    {
        let updates: HashMap<u128, (InfoHash, TorrentUpdateData, UpdatesAction)> = {
            let mut lock = self.torrents_updates.write();
            std::mem::take(&mut *lock)
        };
        if updates.is_empty() {
            return Ok(());
        }
        let drained = updates.len() as i64;
        self.update_stats(StatsEvent::TorrentsUpdates, -drained);
        let mut mapping: HashMap<InfoHash, (u128, TorrentUpdateData, UpdatesAction)> = HashMap::with_capacity(updates.len());
        for (timestamp, (info_hash, torrent_entry, updates_action)) in updates {
            match mapping.entry(info_hash) {
                Entry::Occupied(mut o) => {
                    if timestamp > o.get().0 {
                        o.insert((timestamp, torrent_entry, updates_action));
                    }
                }
                Entry::Vacant(v) => {
                    v.insert((timestamp, torrent_entry, updates_action));
                }
            }
        }
        let mapping_len = mapping.len();
        let is_persistent = torrent_tracker.config.database_structure.torrents.persistent.unwrap_or(torrent_tracker.config.database.persistent);
        let torrents_to_save: BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)> = mapping
            .iter()
            .map(|(info_hash, (_, torrent_update_data, updates_action))| (*info_hash, (*torrent_update_data, *updates_action)))
            .collect();
        let db_result = if is_persistent {
            self.save_torrents(torrent_tracker.clone(), torrents_to_save.clone()).await
        } else {
            Ok(())
        };
        if let Ok(()) = db_result {
            if is_persistent {
                info!("[SYNC TORRENT UPDATES] Synced {mapping_len} torrents");
            }
            if let Some(ref cache) = self.cache {
                let cache_ttl = self.config.cache.as_ref().and_then(|c| {
                    if c.ttl > 0 { Some(c.ttl) } else { None }
                });
                let cache_data: Vec<(InfoHash, TorrentPeerCounts)> = torrents_to_save
                    .iter()
                    .filter(|(_, (_, action))| *action != UpdatesAction::Remove)
                    .map(|(hash, (entry, _))| {
                        let counts = TorrentPeerCounts {
                            bt_seeds_ipv4: entry.seeds_ipv4,
                            bt_seeds_ipv6: entry.seeds_ipv6,
                            rtc_seeds:     entry.rtc_seeds,
                            bt_peers_ipv4: entry.peers_ipv4,
                            bt_peers_ipv6: entry.peers_ipv6,
                            rtc_peers:     entry.rtc_peers,
                            completed:     entry.completed,
                        };
                        (*hash, counts)
                    })
                    .collect();
                if !cache_data.is_empty() {
                    match cache.set_torrent_peers_batch(&cache_data, cache_ttl).await {
                        Ok(()) => {
                            debug!("[Cache] Updated {} torrent peer counts", cache_data.len());
                        }
                        Err(e) => {
                            warn!("[Cache] Failed to update peer counts: {e}");
                        }
                    }
                }
                for (hash, (_, action)) in &torrents_to_save {
                    if *action == UpdatesAction::Remove
                        && let Err(e) = cache.delete_torrent(hash).await {
                            warn!("[Cache] Failed to delete torrent {hash}: {e}");
                        }
                }
            }
            Ok(())
        } else {
            error!("[SYNC TORRENT UPDATES] Unable to sync {mapping_len} torrents");
            let mut lock = self.torrents_updates.write();
            let mut restored = 0i64;
            for (info_hash, (timestamp, torrent_entry, updates_action)) in mapping {
                if let Entry::Vacant(v) = lock.entry(timestamp) {
                    v.insert((info_hash, torrent_entry, updates_action));
                    restored += 1;
                }
            }
            self.update_stats(StatsEvent::TorrentsUpdates, restored);
            Err(())
        }
    }
}