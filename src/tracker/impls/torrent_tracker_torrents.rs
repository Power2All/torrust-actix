use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

impl TorrentTracker {
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok((torrents, completes)) = self.sqlx.load_torrents(tracker).await {
            info!("Loaded {torrents} torrents with {completes} completes");
        }
    }

    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>) -> Result<(), ()>
    {
        let torrents_count = torrents.len();
        match self.sqlx.save_torrents(tracker, torrents).await {
            Ok(_) => {
                info!("[SYNC TORRENTS] Synced {torrents_count} torrents");
                Ok(())
            }
            Err(_) => {
                error!("[SYNC TORRENTS] Unable to sync {torrents_count} torrents");
                Err(())
            }
        }
    }

    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> bool
    {
        match self.sqlx.reset_seeds_peers(tracker).await {
            Ok(_) => {
                info!("[RESET SEEDS PEERS] Completed");
                true
            }
            Err(_) => {
                error!("[RESET SEEDS PEERS] Unable to reset the seeds and peers");
                false
            }
        }
    }

    pub fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) -> (TorrentEntry, bool)
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Torrents, 1);
                self.update_stats(StatsEvent::Completed, torrent_entry.completed as i64);
                self.update_stats(StatsEvent::Seeds, torrent_entry.seeds.len() as i64);
                self.update_stats(StatsEvent::Peers, torrent_entry.peers.len() as i64);
                let entry_clone = torrent_entry.clone();
                v.insert(torrent_entry);
                (entry_clone, true)
            }
            Entry::Occupied(mut o) => {
                let current = o.get_mut();
                let completed_delta = torrent_entry.completed as i64 - current.completed as i64;
                let seeds_delta = torrent_entry.seeds.len() as i64 - current.seeds.len() as i64;
                let peers_delta = torrent_entry.peers.len() as i64 - current.peers.len() as i64;
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
                current.peers = torrent_entry.peers;
                current.updated = torrent_entry.updated;
                (current.clone(), false)
            }
        }
    }

    pub fn add_torrents(&self, hashes: BTreeMap<InfoHash, TorrentEntry>) -> BTreeMap<InfoHash, (TorrentEntry, bool)>
    {
        hashes.into_iter()
            .map(|(info_hash, torrent_entry)| {
                let result = self.add_torrent(info_hash, torrent_entry);
                (info_hash, result)
            })
            .collect()
    }

    #[inline]
    pub fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let lock = shard.read_recursive();
        lock.get(&info_hash).cloned()
    }

    pub fn get_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        hashes.into_iter()
            .map(|info_hash| {
                let entry = self.get_torrent(info_hash);
                (info_hash, entry)
            })
            .collect()
    }

    pub fn remove_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        if !self.torrents_sharding.contains_torrent(info_hash) {
            return None;
        }
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(data) = lock.remove(&info_hash) {
            self.update_stats(StatsEvent::Torrents, -1);
            self.update_stats(StatsEvent::Seeds, -(data.seeds.len() as i64));
            self.update_stats(StatsEvent::Peers, -(data.peers.len() as i64));
            Some(data)
        } else {
            None
        }
    }

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