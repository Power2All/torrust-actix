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
        match self.torrents_map.clone().write().entry(info_hash) {
            Entry::Vacant(v) => {
                (v.insert(torrent_entry).clone(), true)
            }
            Entry::Occupied(o) => {
                (o.get().clone(), false)
            }
        }
    }

    pub fn add_torrents(&self, hashes: BTreeMap<InfoHash, TorrentEntry>) -> BTreeMap<InfoHash, (TorrentEntry, bool)>
    {
        let mut returned_data = BTreeMap::new();
        let map = self.torrents_map.clone();
        let mut lock = map.write();
        for (info_hash, torrent_entry) in hashes.iter() {
            match lock.entry(*info_hash) {
                Entry::Vacant(v) => {
                    returned_data.insert(*info_hash, (torrent_entry.clone(), true));
                    v.insert(torrent_entry.clone());
                }
                Entry::Occupied(mut o) => {
                    returned_data.insert(*info_hash, (o.get().clone(), false));
                    o.insert(torrent_entry.clone());
                }
            }
        }
        returned_data
    }

    pub fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        match self.torrents_map.clone().read().get(&info_hash) {
            None => { None }
            Some(torrent) => {
                Some(TorrentEntry {
                    seeds: torrent.seeds.clone(),
                    peers: torrent.peers.clone(),
                    completed: torrent.completed,
                    updated: torrent.updated,
                })
            }
        }
    }

    pub fn get_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        let map = self.torrents_map.clone();
        let lock = map.read();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, lock.get(info_hash).map(|torrent| torrent.clone()));
        }
        returned_data
    }

    pub fn get_torrents_chunk(&self, skip: usize, amount: usize) -> BTreeMap<InfoHash, TorrentEntry>
    {
        self.torrents_map.clone().read().iter().skip(skip).into_iter().take(amount).map(|(info_hash, torrent_entry)| (info_hash.clone(), torrent_entry.clone())).collect::<BTreeMap<InfoHash, TorrentEntry>>()
    }

    pub fn remove_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        self.torrents_map.clone().write().remove(&info_hash)
    }

    pub fn remove_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        let map = self.torrents_map.clone();
        let mut lock = map.write();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, lock.remove(info_hash).map(|torrent| torrent.clone()));
        }
        returned_data
    }
}
