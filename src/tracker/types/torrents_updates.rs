use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;
use crate::tracker::types::ahash_map::AHashMap;
use parking_lot::RwLock;
use std::collections::hash_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Number of independently locked shards, matching the torrent store.
const SHARDS: usize = 256;

/// A pending torrent update: the sequence number that ordered it, the counters to write and
/// what to do with them.
pub type PendingUpdate = (u128, TorrentUpdateData, UpdatesAction);

/// Queue of torrent stat changes awaiting the next database/cache flush.
///
/// Sharded by the first byte of the info-hash, like the torrent store, so the announce path does
/// not serialise on one lock. Entries are keyed by info-hash and deduplicated on insert, highest
/// sequence number winning, which bounds the queue by distinct torrents touched rather than by
/// announces served between flushes.
#[derive(Debug)]
pub struct TorrentUpdateQueue {
    shards: [RwLock<AHashMap<InfoHash, PendingUpdate>>; SHARDS],
}

impl Default for TorrentUpdateQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl TorrentUpdateQueue {
    /// Creates an empty queue with 256 independently locked shards.
    pub fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| RwLock::new(AHashMap::default())),
        }
    }

    #[inline]
    fn shard(&self, info_hash: InfoHash) -> &RwLock<AHashMap<InfoHash, PendingUpdate>> {
        &self.shards[info_hash.0[0] as usize]
    }

    /// Queues an update for `info_hash`, replacing any pending update with a lower sequence
    /// number.
    ///
    /// Returns `true` when this created a new queue slot, so the caller can keep the
    /// `torrents_updates` statistic equal to the number of pending entries.
    #[inline]
    pub fn insert(&self, seq: u128, info_hash: InfoHash, data: TorrentUpdateData, action: UpdatesAction) -> bool {
        let mut lock = self.shard(info_hash).write();
        match lock.entry(info_hash) {
            Entry::Occupied(mut o) => {
                // Threads can take sequence numbers in one order and reach this lock in another,
                // so the later writer is not necessarily the newer update.
                if seq > o.get().0 {
                    o.insert((seq, data, action));
                }
                false
            }
            Entry::Vacant(v) => {
                v.insert((seq, data, action));
                true
            }
        }
    }

    /// Removes and returns every pending update.
    pub fn drain(&self) -> BTreeMap<InfoHash, PendingUpdate> {
        let mut drained = BTreeMap::new();
        for shard in &self.shards {
            let taken = std::mem::take(&mut *shard.write());
            drained.extend(taken);
        }
        drained
    }

    /// Puts drained updates back after a failed flush, skipping any info-hash that has since
    /// been given a newer update.
    ///
    /// Returns how many entries were restored.
    pub fn restore(&self, updates: BTreeMap<InfoHash, PendingUpdate>) -> i64 {
        let mut restored = 0i64;
        for (info_hash, (seq, data, action)) in updates {
            if self.insert(seq, info_hash, data, action) {
                restored += 1;
            }
        }
        restored
    }

    /// Returns a copy of the pending updates without removing them.
    pub fn snapshot(&self) -> BTreeMap<InfoHash, PendingUpdate> {
        let mut out = BTreeMap::new();
        for shard in &self.shards {
            out.extend(shard.read().iter().map(|(k, v)| (*k, *v)));
        }
        out
    }

    /// Drops every pending update.
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.write().clear();
        }
    }

    /// Returns the number of pending updates across all shards.
    pub fn len(&self) -> usize {
        self.shards.iter().map(|shard| shard.read().len()).sum()
    }

    /// Returns `true` when no update is pending.
    pub fn is_empty(&self) -> bool {
        self.shards.iter().all(|shard| shard.read().is_empty())
    }
}

pub type TorrentsUpdates = Arc<TorrentUpdateQueue>;
