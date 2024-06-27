use log::info;
use async_std::sync::Arc;
use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;

use crate::common::{InfoHash, PeerId, TorrentPeer};
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentEntryItem {
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}

impl TorrentEntryItem {
    pub fn new() -> TorrentEntryItem {
        TorrentEntryItem {
            completed: 0,
            seeders: 0,
            leechers: 0,
        }
    }
}

impl Default for TorrentEntryItem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentEntry {
    #[serde(skip)]
    pub peers: BTreeMap<PeerId, TorrentPeer>,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}

impl TorrentEntry {
    pub fn new() -> TorrentEntry {
        TorrentEntry {
            peers: BTreeMap::new(),
            completed: 0,
            seeders: 0,
            leechers: 0,
        }
    }
}

impl Default for TorrentEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetTorrentsApi {
    pub info_hash: String,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetTorrentApi {
    pub info_hash: String,
    pub completed: i64,
    pub seeders: i64,
    pub leechers: i64,
    pub peers: Vec<Value>,
}

impl TorrentTracker {
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok((torrents, completes)) = self.sqlx.load_torrents(tracker.clone()).await {
            info!("Loaded {} torrents with {} completes.", torrents, completes);
            self.set_stats(StatsEvent::Completed, completes as i64).await;
        }
    }

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntryItem, persistent: bool)
    {
        let torrents_arc = self.torrents.clone();

        torrents_arc.insert(info_hash, torrent_entry.clone());

        if persistent { self.add_torrents_update(info_hash, torrent_entry.completed).await; }

        self.update_stats(StatsEvent::Torrents, 1).await;
    }

    pub async fn add_torrents(&self, torrents: HashMap<InfoHash, TorrentEntryItem>, persistent: bool)
    {
        let torrents_arc = self.torrents.clone();

        let mut updates = HashMap::new();
        for (info_hash, torrent_entry) in torrents.iter() {
            torrents_arc.insert(*info_hash, torrent_entry.clone());
            updates.insert(*info_hash, torrent_entry.completed);
        }

        if persistent { self.add_torrents_updates(updates).await; }

        self.update_stats(StatsEvent::Torrents, torrents.len() as i64).await;
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let torrents_arc = self.torrents.clone();
        let peers_arc = self.peers.clone();

        let torrent = match torrents_arc.get(&info_hash) {
            None => { None }
            Some(data) => {
                let peers = match peers_arc.get(&info_hash) {
                    None => { BTreeMap::new() }
                    Some(data) => { data.value().clone() }
                };
                Some(TorrentEntry {
                    peers,
                    completed: data.value().clone().completed,
                    seeders: data.value().clone().seeders,
                    leechers: data.value().clone().leechers,
                })
            }
        };

        torrent
    }

    pub async fn get_torrents(&self, hashes: Vec<InfoHash>) -> HashMap<InfoHash, Option<TorrentEntry>>
    {
        let torrents_arc = self.torrents.clone();
        let peers_arc = self.peers.clone();

        let mut return_torrents = HashMap::new();

        for info_hash in hashes.iter() {
            return_torrents.insert(*info_hash, match torrents_arc.get(info_hash) {
                None => { None }
                Some(data_torrent) => {
                    let data = data_torrent.value().clone();
                    let peers = match peers_arc.get(info_hash) {
                        None => { BTreeMap::new() }
                        Some(data_peers) => { data_peers.value().clone() }
                    };
                    Some(TorrentEntry {
                        peers,
                        completed: data.completed,
                        seeders: data.seeders,
                        leechers: data.leechers,
                    })
                }
            });
        }

        return_torrents
    }

    pub async fn get_torrents_chunk(&self, skip: u64, amount: u64) -> HashMap<InfoHash, i64>
    {
        let torrents_arc = self.torrents.clone();

        let mut torrents_return: HashMap<InfoHash, i64> = HashMap::new();
        let mut current_count: u64 = 0;
        let mut handled_count: u64 = 0;
        for item in torrents_arc.iter() {
            if current_count < skip {
                current_count = current_count.add(1);
                continue;
            }
            if handled_count >= amount { break; }
            torrents_return.insert(*item.key(), item.value().clone().completed);
            current_count = current_count.add(1);
            handled_count = handled_count.add(1);
        }

        torrents_return
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool)
    {
        let torrents_arc = self.torrents.clone();
        let peers_arc = self.peers.clone();

        let mut removed_torrent = false;
        let mut remove_seeders = 0i64;
        let mut remove_leechers = 0i64;

        match torrents_arc.remove(&info_hash) {
            None => {}
            Some(data) => {
                removed_torrent = true;
                remove_seeders -= data.value().seeders;
                remove_leechers -= data.value().leechers;
            }
        }
        peers_arc.remove(&info_hash);

        if persistent {
            self.remove_torrents_update(info_hash).await;
            self.remove_torrents_shadow(info_hash).await;
        }

        if removed_torrent { self.update_stats(StatsEvent::Torrents, -1).await; }
        if remove_seeders != 0 { self.update_stats(StatsEvent::Seeds, remove_seeders).await; }
        if remove_leechers != 0 { self.update_stats(StatsEvent::Peers, remove_leechers).await; }
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>, persistent: bool)
    {
        let torrents_arc = self.torrents.clone();

        for info_hash in hashes.iter() { if torrents_arc.get(info_hash).is_some() { self.remove_torrent(*info_hash, persistent).await; } }
    }
}