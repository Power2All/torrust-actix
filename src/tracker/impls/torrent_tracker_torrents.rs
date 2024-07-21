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
            self.set_stats(StatsEvent::Completed, completes as i64);
        }
    }

    pub fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) -> (TorrentEntry, bool)
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        let return_data = match lock.entry(info_hash) {
            Entry::Vacant(v) => {
                self.update_stats(StatsEvent::Torrents, 1);
                (v.insert(torrent_entry).clone(), true)
            }
            Entry::Occupied(o) => {
                (o.get().clone(), false)
            }
        };
        drop(lock);
        drop(shard);
        return_data
    }

    pub fn add_torrents(&self, hashes: BTreeMap<InfoHash, TorrentEntry>) -> BTreeMap<InfoHash, (TorrentEntry, bool)>
    {
        let mut returned_data = BTreeMap::new();
        for (info_hash, torrent_entry) in hashes.iter() {
            returned_data.insert(*info_hash, self.add_torrent(*info_hash, torrent_entry.clone()));
        }
        returned_data
    }

    pub fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let lock = shard.read();
        let torrent = lock.get(&info_hash).map(|torrent| TorrentEntry {
            seeds: torrent.seeds.clone(),
            peers: torrent.peers.clone(),
            completed: torrent.completed,
            updated: torrent.updated
        });
        drop(lock);
        drop(shard);
        torrent
    }

    pub fn get_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, self.get_torrent(*info_hash));
        }
        returned_data
    }

    pub fn remove_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        let return_data = match lock.remove(&info_hash) {
            None => { None }
            Some(data) => {
                self.update_stats(StatsEvent::Torrents, -1);
                self.update_stats(StatsEvent::Seeds, data.seeds.len() as i64);
                self.update_stats(StatsEvent::Peers, data.peers.len() as i64);
                Some(data)
            }
        };
        drop(lock);
        drop(shard);
        return_data
    }

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
