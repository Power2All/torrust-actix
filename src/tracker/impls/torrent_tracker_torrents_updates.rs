use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
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
use parking_lot::RwLock;
use std::collections::{
    BTreeMap,
    HashMap
};
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic, process-local sequence number used as the dedupe ordering
/// key for every queued update (torrents, keys, users, whitelist, blacklist).
/// Replaces wall-clock nanoseconds (`SystemTime::now()`), which can jump
/// backwards on NTP corrections / leap seconds and would let an older update
/// win the dedupe, and which two updates in one clock tick (100 ns on Windows)
/// share, the second silently replacing the first.
static UPDATE_SEQ: AtomicU64 = AtomicU64::new(0);

#[inline]
pub(crate) fn next_seq() -> u128 {
    u128::from(UPDATE_SEQ.fetch_add(1, Ordering::Relaxed))
}

impl TorrentTracker {
    /// Loads the whitelist, blacklist, keys and users at startup. A list is loaded when its mode is
    /// on (with `[database] persistent`), or when its table is persistent (written back by
    /// [`Self::save_all_updates`]) so it can be filled before the mode is switched on.
    /// `persistent = false` only stops write-back: a read-only whitelist pointed at the torrents
    /// table must still be loaded while whitelist mode is on.
    pub async fn load_lists(tracker: &Arc<TorrentTracker>)
    {
        let modes = &tracker.config.tracker_config;
        let structure = &tracker.config.database_structure;
        let persistent = tracker.config.database.persistent;
        let wanted = |enabled: bool, table: Option<bool>| (persistent && enabled) || table.unwrap_or(persistent);
        if wanted(modes.whitelist_enabled, structure.whitelist.persistent) {
            tracker.load_whitelist(Arc::clone(tracker)).await;
        }
        if wanted(modes.blacklist_enabled, structure.blacklist.persistent) {
            tracker.load_blacklist(Arc::clone(tracker)).await;
        }
        if wanted(modes.keys_enabled, structure.keys.persistent) {
            tracker.load_keys(Arc::clone(tracker)).await;
        }
        if wanted(modes.users_enabled, structure.users.persistent) {
            tracker.load_users(Arc::clone(tracker)).await;
        }
    }

    /// Flushes every update queue, each in its own task: torrents always and users with a cache
    /// (both also feed the cache), and the whitelist, blacklist, keys and users when their table
    /// is persistent. That
    /// is independent of whether their mode is on, which only decides whether announces are checked
    /// against them. Each flush logs its own failure and keeps its queue for the next run.
    pub async fn save_all_updates(tracker: &Arc<TorrentTracker>)
    {
        macro_rules! flush {
            ($save:ident) => {{
                let tracker = Arc::clone(tracker);
                tokio::spawn(async move {
                    let _ = tracker.$save(Arc::clone(&tracker)).await;
                })
            }};
        }
        let structure = &tracker.config.database_structure;
        let persistent = tracker.config.database.persistent;
        let mut tasks = vec![flush!(save_torrent_updates)];
        if structure.whitelist.persistent.unwrap_or(persistent) {
            tasks.push(flush!(save_whitelist_updates));
        }
        if structure.blacklist.persistent.unwrap_or(persistent) {
            tasks.push(flush!(save_blacklist_updates));
        }
        if structure.keys.persistent.unwrap_or(persistent) {
            tasks.push(flush!(save_key_updates));
        }
        // Users also feed the cache (`<prefix>u:<id>`), so they flush without a persistent table too.
        if structure.users.persistent.unwrap_or(persistent) || tracker.cache.is_some() {
            tasks.push(flush!(save_user_updates));
        }
        for task in tasks {
            let _ = task.await;
        }
    }

    /// Removes a flushed snapshot from its update queue and lowers `event` by the count removed.
    /// An entry whose sequence number changed while the flush ran was replaced by a newer update,
    /// so it stays for the next flush.
    pub(crate) fn retire_flushed<K: Hash + Eq, V>(&self, queue: &RwLock<HashMap<K, V>>, flushed: &HashMap<K, V>, seq: impl Fn(&V) -> u128, event: StatsEvent)
    {
        let mut lock = queue.write();
        let mut removed_count = 0i64;
        for (key, value) in flushed {
            if lock.get(key).is_some_and(|current| seq(current) == seq(value)) {
                lock.remove(key);
                removed_count += 1;
            }
        }
        if removed_count > 0 {
            self.update_stats(event, -removed_count);
        }
    }
}

impl TorrentTracker {
    /// Queues a torrent update for the next database/cache flush.
    ///
    /// Returns `true` when a new queue slot was created; a `false` means an update for the same
    /// info-hash was already pending and has been superseded.
    pub fn add_torrent_update(&self, info_hash: InfoHash, torrent_update_data: TorrentUpdateData, updates_action: UpdatesAction) -> bool
    {
        if self.torrents_updates.insert(next_seq(), info_hash, torrent_update_data, updates_action) {
            self.update_stats(StatsEvent::TorrentsUpdates, 1);
            true
        } else {
            false
        }
    }

    /// Returns a copy of the pending torrent-update queue, keyed by info-hash.
    pub fn get_torrent_updates(&self) -> BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)>
    {
        self.torrents_updates.snapshot()
            .into_iter()
            .map(|(info_hash, (_, data, action))| (info_hash, (data, action)))
            .collect()
    }

    /// Drops all queued torrent updates and resets the queue statistic.
    pub fn clear_torrent_updates(&self)
    {
        self.torrents_updates.clear();
        self.set_stats(StatsEvent::TorrentsUpdates, 0);
    }

    /// Drains the update queue, deduplicates it per info-hash (newest wins) and flushes the
    /// result to the database and/or peer-count cache.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database flush fails; the drained updates are restored to the
    /// queue so no data is lost.
    pub async fn save_torrent_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), sqlx::Error>
    {
        let drained_updates = self.torrents_updates.drain();
        if drained_updates.is_empty() {
            return Ok(());
        }
        let mapping_len = drained_updates.len();
        self.update_stats(StatsEvent::TorrentsUpdates, -(mapping_len as i64));
        let is_persistent = torrent_tracker.config.database_structure.torrents.persistent.unwrap_or(torrent_tracker.config.database.persistent);
        let torrents_to_save: BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)> = drained_updates
            .iter()
            .map(|(info_hash, (_, torrent_update_data, updates_action))| (*info_hash, (*torrent_update_data, *updates_action)))
            .collect();
        let db_result = if is_persistent {
            self.sqlx.save_torrents(torrent_tracker.clone(), &torrents_to_save).await
        } else {
            Ok(())
        };
        // The cache flush is deliberately *not* gated on `db_result`: the two are
        // independent sinks, and while the database was unreachable this used to
        // stop refreshing the peer counts in Redis/memcache as well, so scrapes
        // served stale numbers for as long as the outage lasted. Writing absolute
        // counts is idempotent, so a batch the database rejected and re-queued
        // simply gets written to the cache again on the next flush.
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
            let removals: Vec<InfoHash> = torrents_to_save
                .iter()
                .filter(|(_, (_, action))| *action == UpdatesAction::Remove)
                .map(|(hash, _)| *hash)
                .collect();
            if !removals.is_empty()
                && let Err(e) = cache.delete_torrents(&removals).await {
                    warn!("[Cache] Failed to delete {} torrents: {e}", removals.len());
                }
        }
        match db_result {
            Ok(()) => {
                if is_persistent {
                    info!("[SYNC TORRENT UPDATES] Synced {mapping_len} torrents");
                }
                Ok(())
            }
            Err(e) => {
                error!("[SYNC TORRENT UPDATES] Unable to sync {mapping_len} torrents, re-queued for the next flush: {e}");
                let restored = self.torrents_updates.restore(drained_updates);
                self.update_stats(StatsEvent::TorrentsUpdates, restored);
                Err(e)
            }
        }
    }
}