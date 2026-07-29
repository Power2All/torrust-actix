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
use std::collections::BTreeMap;
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

    /// Queues a batch of torrent updates.
    ///
    /// Returns, per info-hash, whether the insert created a new slot.
    pub fn add_torrent_updates(&self, hashes: BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)>) -> BTreeMap<InfoHash, bool>
    {
        let mut returned_data = BTreeMap::new();
        let mut success_count = 0i64;
        for (info_hash, (torrent_entry, updates_action)) in hashes {
            let success = self.torrents_updates.insert(next_seq(), info_hash, torrent_entry, updates_action);
            if success {
                success_count += 1;
            }
            returned_data.insert(info_hash, success);
        }
        if success_count > 0 {
            self.update_stats(StatsEvent::TorrentsUpdates, success_count);
        }
        returned_data
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
    pub async fn save_torrent_updates(&self, torrent_tracker: Arc<TorrentTracker>) -> Result<(), ()>
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
            self.save_torrents(torrent_tracker.clone(), &torrents_to_save).await
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
        if let Ok(()) = db_result {
            if is_persistent {
                info!("[SYNC TORRENT UPDATES] Synced {mapping_len} torrents");
            }
            Ok(())
        } else {
            error!("[SYNC TORRENT UPDATES] Unable to sync {mapping_len} torrents, re-queued for the next flush");
            let restored = self.torrents_updates.restore(drained_updates);
            self.update_stats(StatsEvent::TorrentsUpdates, restored);
            Err(())
        }
    }
}