use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;
use log::info;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok((torrents, completes)) = self.sqlx.load_torrents(tracker.clone()).await {
            info!("Loaded {} torrents with {} completes.", torrents, completes);
            self.set_stats(StatsEvent::Completed, completes as i64).await;
        }
    }

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry, persistent: bool) -> TorrentEntry
    {
        let map = self.torrents_map.clone();
        let mut lock = map.write();
        let torrent = match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Torrents, 1).await;
                v.insert(torrent_entry.clone()).clone()
            }
            Entry::Occupied(o) => {
                o.get().clone()
            }
        };
        if persistent {
            self.add_torrents_update(info_hash, torrent_entry.completed as i64).await;
        }
        torrent.clone()
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let map = self.torrents_map.clone();
        let lock = map.read();
        match lock.get(&info_hash) {
            None => { None }
            Some(t) => {
                Some(TorrentEntry {
                    seeds: t.seeds.clone(),
                    peers: t.peers.clone(),
                    completed: t.completed,
                    updated: t.updated
                })
            }
        }
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool) -> (bool, u64, u64)
    {
        let map = self.torrents_map.clone();
        let mut lock = map.write();
        let result = match lock.entry(info_hash) {
            Entry::Vacant(_) => {
                (false, 0, 0)
            }
            Entry::Occupied(o) => {
                let seeds = o.get().clone().seeds.len();
                let peers = o.get().clone().peers.len();
                o.remove();
                (true, seeds as u64, peers as u64)
            }
        };
        if result.0 {
            self.update_stats(StatsEvent::Seeds, 0 - result.1 as i64).await;
            self.update_stats(StatsEvent::Peers, 0 - result.2 as i64).await;
        }
        result
    }

    pub async fn get_torrents_chunk(&self, skip: usize, amount: usize) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let lock = self.torrents_map.clone();
        let mut count = 0usize;
        let mut returned_data = BTreeMap::new();
        if lock.read().len() > skip {
            return returned_data;
        }
        for (info_hash, torrent_entry) in lock.read().iter().skip(skip) {
            count += 1;
            if count == amount {
                break;
            }
            returned_data.insert(*info_hash, TorrentEntry {
                seeds: torrent_entry.seeds.clone(),
                peers: torrent_entry.peers.clone(),
                completed: torrent_entry.completed,
                updated: torrent_entry.updated
            });
        }
        returned_data
    }
}
