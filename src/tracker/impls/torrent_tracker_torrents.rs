use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;
use log::{
    error,
    info
};
use std::collections::hash_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

impl TorrentTracker {
    /// Loads all torrents (and their completion counts) from the configured database at startup.
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok((torrents, completes)) = self.sqlx.load_torrents(tracker).await {
            info!("Loaded {torrents} torrents with {completes} completes");
        }
    }

    /// Persists the given batch of torrent updates to the database.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the database write fails; the caller re-queues the batch.
    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)>) -> Result<(), ()>
    {
        let torrents_count = torrents.len();
        if let Ok(()) = self.sqlx.save_torrents(tracker, torrents).await {
            info!("[SYNC TORRENTS] Synced {torrents_count} torrents");
            Ok(())
        } else {
            error!("[SYNC TORRENTS] Unable to sync {torrents_count} torrents");
            Err(())
        }
    }

    /// Resets the seed and peer counters of every torrent row in the database.
    ///
    /// Used at startup so stale counts from a previous run do not linger. Returns `true` on success.
    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> bool
    {
        if let Ok(()) = self.sqlx.reset_seeds_peers(tracker).await {
            info!("[RESET SEEDS PEERS] Completed");
            true
        } else {
            error!("[RESET SEEDS PEERS] Unable to reset the seeds and peers");
            false
        }
    }

    /// Inserts or replaces a full torrent entry, adjusting the global torrent/seed/peer/completed
    /// statistics by the delta between the old and new entry.
    ///
    /// Returns the stored entry and `true` when the torrent was newly inserted.
    pub fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) -> (TorrentEntry, bool)
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Torrents, 1);
                self.update_stats(StatsEvent::Completed, torrent_entry.completed as i64);
                self.update_stats(StatsEvent::Seeds, (torrent_entry.seeds.len() + torrent_entry.seeds_ipv6.len()) as i64);
                self.update_stats(StatsEvent::Peers, (torrent_entry.peers.len() + torrent_entry.peers_ipv6.len()) as i64);
                let entry_clone = torrent_entry.clone();
                v.insert(torrent_entry);
                (entry_clone, true)
            }
            Entry::Occupied(mut o) => {
                let current = o.get_mut();
                let completed_delta = torrent_entry.completed as i64 - current.completed as i64;
                let seeds_delta = (torrent_entry.seeds.len() + torrent_entry.seeds_ipv6.len()) as i64
                    - (current.seeds.len() + current.seeds_ipv6.len()) as i64;
                let peers_delta = (torrent_entry.peers.len() + torrent_entry.peers_ipv6.len()) as i64
                    - (current.peers.len() + current.peers_ipv6.len()) as i64;
                if completed_delta != 0 {
                    self.update_stats(StatsEvent::Completed, completed_delta);
                }
                if seeds_delta != 0 {
                    self.update_stats(StatsEvent::Seeds, seeds_delta);
                }
                if peers_delta != 0 {
                    self.update_stats(StatsEvent::Peers, peers_delta);
                }
                current.completed = torrent_entry.completed;
                current.seeds = torrent_entry.seeds;
                current.seeds_ipv6 = torrent_entry.seeds_ipv6;
                current.peers = torrent_entry.peers;
                current.peers_ipv6 = torrent_entry.peers_ipv6;
                current.rtc_seeds = torrent_entry.rtc_seeds;
                current.rtc_peers = torrent_entry.rtc_peers;
                current.updated = torrent_entry.updated;
                (current.clone(), false)
            }
        }
    }

    /// Inserts or replaces multiple torrent entries; see [`TorrentTracker::add_torrent`].
    pub fn add_torrents(&self, hashes: BTreeMap<InfoHash, TorrentEntry>) -> BTreeMap<InfoHash, (TorrentEntry, bool)>
    {
        hashes.into_iter()
            .map(|(info_hash, torrent_entry)| {
                let result = self.add_torrent(info_hash, torrent_entry);
                (info_hash, result)
            })
            .collect()
    }

    /// Returns a full clone of the torrent entry, including all peer maps.
    ///
    /// Prefer [`TorrentTracker::get_torrent_counts`] when only counters are needed.
    #[inline]
    pub fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let lock = shard.read_recursive();
        lock.get(&info_hash).cloned()
    }

    /// Returns only the seed/peer/completed counters of a torrent, without cloning its peer maps.
    ///
    /// This is the cheap lookup used by scrape handling.
    #[inline]
    pub fn get_torrent_counts(&self, info_hash: InfoHash) -> Option<crate::tracker::structs::torrent_counts::TorrentCounts>
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let lock = shard.read_recursive();
        lock.get(&info_hash).map(crate::tracker::structs::torrent_counts::TorrentCounts::from_entry)
    }

    /// Returns full clones of multiple torrent entries; absent torrents map to `None`.
    pub fn get_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        hashes.into_iter()
            .map(|info_hash| {
                let entry = self.get_torrent(info_hash);
                (info_hash, entry)
            })
            .collect()
    }

    /// Removes a torrent and subtracts its seeds/peers from the global statistics.
    ///
    /// Returns the removed entry if it existed.
    pub fn remove_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        if !self.torrents_sharding.contains_torrent(info_hash) {
            return None;
        }
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(data) = lock.remove(&info_hash) {
            self.update_stats(StatsEvent::Torrents, -1);
            self.update_stats(StatsEvent::Seeds, -((data.seeds.len() + data.seeds_ipv6.len()) as i64));
            self.update_stats(StatsEvent::Peers, -((data.peers.len() + data.peers_ipv6.len()) as i64));
            Some(data)
        } else {
            None
        }
    }

    /// Removes multiple torrents; see [`TorrentTracker::remove_torrent`].
    pub fn remove_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        hashes.into_iter()
            .map(|info_hash| {
                let result = self.remove_torrent(info_hash);
                (info_hash, result)
            })
            .collect()
    }
}