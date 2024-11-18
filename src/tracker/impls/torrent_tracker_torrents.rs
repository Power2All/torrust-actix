use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;
use log::{error, info};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument]
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok((torrents, completes)) = self.sqlx.load_torrents(tracker.clone()).await {
            info!("Loaded {} torrents with {} completes", torrents, completes);
        }
    }

    #[tracing::instrument]
    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>) -> Result<(), ()>
    {
        match self.sqlx.save_torrents(tracker.clone(), torrents.clone()).await {
            Ok(_) => {
                info!("[SYNC TORRENTS] Synced {} torrents", torrents.len());
                Ok(())
            }
            Err(_) => {
                error!("[SYNC TORRENTS] Unable to sync {} torrents", torrents.len());
                Err(())
            }
        }
    }

    #[tracing::instrument]
    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> bool
    {
        match self.sqlx.reset_seeds_peers(tracker.clone()).await {
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

    #[tracing::instrument]
    pub fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) -> (TorrentEntry, bool)
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Torrents, 1);
                self.update_stats(StatsEvent::Completed, torrent_entry.completed as i64);
                self.update_stats(StatsEvent::Seeds, torrent_entry.seeds.len() as i64);
                self.update_stats(StatsEvent::Peers, torrent_entry.peers.len() as i64);
                (v.insert(torrent_entry).clone(), true)
            }
            Entry::Occupied(mut o) => {
                self.update_stats(StatsEvent::Completed, 0i64 - o.get().completed as i64);
                self.update_stats(StatsEvent::Completed, torrent_entry.completed as i64);
                o.get_mut().completed = torrent_entry.completed;
                self.update_stats(StatsEvent::Seeds, 0i64 - o.get().seeds.len() as i64);
                self.update_stats(StatsEvent::Seeds, torrent_entry.seeds.len() as i64);
                o.get_mut().seeds = torrent_entry.seeds.clone();
                self.update_stats(StatsEvent::Peers, 0i64 - o.get().peers.len() as i64);
                self.update_stats(StatsEvent::Peers, torrent_entry.peers.len() as i64);
                o.get_mut().peers = torrent_entry.peers.clone();
                o.get_mut().updated = torrent_entry.updated;
                (torrent_entry.clone(), false)
            }
        }
    }

    #[tracing::instrument]
    pub fn add_torrents(&self, hashes: BTreeMap<InfoHash, TorrentEntry>) -> BTreeMap<InfoHash, (TorrentEntry, bool)>
    {
        let mut returned_data = BTreeMap::new();
        for (info_hash, torrent_entry) in hashes.iter() {
            returned_data.insert(*info_hash, self.add_torrent(*info_hash, torrent_entry.clone()));
        }
        returned_data
    }

    #[tracing::instrument]
    pub fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let lock = shard.read_recursive();
        lock.get(&info_hash).map(|torrent| TorrentEntry {
            seeds: torrent.seeds.clone(),
            peers: torrent.peers.clone(),
            completed: torrent.completed,
            updated: torrent.updated
        })
    }

    #[tracing::instrument]
    pub fn get_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, self.get_torrent(*info_hash));
        }
        returned_data
    }

    #[tracing::instrument]
    pub fn remove_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        match lock.remove(&info_hash) {
            None => { None }
            Some(data) => {
                self.update_stats(StatsEvent::Torrents, -1);
                self.update_stats(StatsEvent::Seeds, data.seeds.len() as i64);
                self.update_stats(StatsEvent::Peers, data.peers.len() as i64);
                Some(data)
            }
        }
    }

    #[tracing::instrument]
    pub fn remove_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, match self.remove_torrent(*info_hash) {
                None => { None }
                Some(torrent) => {
                    self.update_stats(StatsEvent::Torrents, -1);
                    self.update_stats(StatsEvent::Seeds, torrent.seeds.len() as i64);
                    self.update_stats(StatsEvent::Peers, torrent.peers.len() as i64);
                    Some(torrent)
                }
            });
        }
        returned_data
    }
}