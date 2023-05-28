use async_std::future::timeout;
use log::{error, info};
use scc::ebr::Arc;
use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::time::Duration;

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
        let torrents_arc = self.map_torrents.clone();
        let mut torrents_lock = torrents_arc.lock().await;
        let _ = torrents_lock.insert(info_hash, torrent_entry.clone());
        drop(torrents_lock);

        if persistent {
            self.add_update(
                info_hash,
                torrent_entry.completed,
            ).await;
        }

        self.update_stats(StatsEvent::Torrents, 1).await;
    }

    pub async fn add_torrents(&self, torrents: HashMap<InfoHash, TorrentEntryItem>, persistent: bool)
    {
        let torrents_arc = self.map_torrents.clone();
        let mut torrents_lock = torrents_arc.lock().await;
        let mut updates = HashMap::new();
        for (info_hash, torrent_entry) in torrents.iter() {
            let _ = torrents_lock.insert(*info_hash, torrent_entry.clone());
            updates.insert(*info_hash, torrent_entry.completed);
        }
        drop(torrents_lock);

        if persistent {
            self.add_updates(
                updates
            ).await;
        }

        self.update_stats(StatsEvent::Torrents, torrents.len() as i64).await;
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Result<Option<TorrentEntry>, ()>
    {
        let torrent = match timeout(Duration::from_secs(30), async move {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.lock().await;
            let torrent = torrents_lock.get(&info_hash).cloned();
            drop(torrents_lock);
            torrent
        }).await {
            Ok(data) => { data }
            Err(_) => { error!("[GET_TORRENT] Read Lock (torrents) request timed out!"); return Err(()); }
        };

        let torrent = match torrent {
            None => { None }
            Some(data) => {
                let peers = match timeout(Duration::from_secs(30), async move {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.lock().await;
                    let peers = match peers_lock.get(&info_hash).cloned() {
                        None => { BTreeMap::new() }
                        Some(data) => { data }
                    };
                    drop(peers_lock);
                    peers
                }).await {
                    Ok(data) => { data }
                    Err(_) => { error!("[GET_TORRENT] Read Lock (peers) request timed out!"); return Err(()); }
                };
                Some(TorrentEntry {
                    peers,
                    completed: data.completed,
                    seeders: data.seeders,
                    leechers: data.leechers,
                })
            }
        };

        Ok(torrent)
    }

    pub async fn get_torrents(&self, hashes: Vec<InfoHash>) -> Result<HashMap<InfoHash, Option<TorrentEntry>>, ()>
    {
        let mut return_torrents = HashMap::new();

        for info_hash in hashes.iter() {
            let torrent = match timeout(Duration::from_secs(30), async move {
                let torrents_arc = self.map_torrents.clone();
                let torrents_lock = torrents_arc.lock().await;
                let torrent = torrents_lock.get(info_hash).cloned();
                drop(torrents_lock);
                torrent
            }).await {
                Ok(data) => { data }
                Err(_) => { error!("[GET_TORRENTS] Read Lock (torrents) request timed out!"); return Err(()); }
            };

            return_torrents.insert(*info_hash, match torrent {
                None => { None }
                Some(data) => {
                    let peers = match timeout(Duration::from_secs(30), async move {
                        let peers_arc = self.map_peers.clone();
                        let peers_lock = peers_arc.lock().await;
                        let peers_data = match peers_lock.get(info_hash).cloned() {
                            None => { BTreeMap::new() }
                            Some(data_peers) => { data_peers }
                        };
                        drop(peers_lock);
                        peers_data
                    }).await {
                        Ok(data_peers) => { data_peers }
                        Err(_) => { error!("[GET_TORRENTS] Read Lock (peers) request timed out!"); return Err(()); }
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

        Ok(return_torrents)
    }

    pub async fn get_torrents_chunk(&self, skip: u64, amount: u64) -> Result<HashMap<InfoHash, i64>, ()>
    {
        match timeout(Duration::from_secs(30), async move {
            let torrents_arc = self.map_torrents.clone();
            let torrents_lock = torrents_arc.lock().await;
            let mut torrents_return: HashMap<InfoHash, i64> = HashMap::new();
            let mut current_count: u64 = 0;
            let mut handled_count: u64 = 0;
            for (info_hash, item) in torrents_lock.iter() {
                if current_count < skip {
                    current_count = current_count.add(1);
                    continue;
                }
                if handled_count >= amount {
                    break;
                }
                torrents_return.insert(*info_hash, item.completed);
                current_count = current_count.add(1);
                handled_count = handled_count.add(1);
            }
            drop(torrents_lock);
            torrents_return
        }).await {
            Ok(data) => { Ok(data) }
            Err(_) => { Err(()) }
        }
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool)
    {
        let mut removed_torrent = false;
        let mut remove_seeders = 0i64;
        let mut remove_leechers = 0i64;

        let torrents_arc = self.map_torrents.clone();
        let mut torrents_lock = torrents_arc.lock().await;
        let torrent_option = torrents_lock.get(&info_hash);
        match torrent_option {
            None => {}
            Some(data) => {
                removed_torrent = true;
                remove_seeders -= data.seeders;
                remove_leechers -= data.leechers;
                torrents_lock.remove(&info_hash);
            }
        }
        drop(torrents_lock);

        let peers_arc = self.map_peers.clone();
        let mut peers_lock = peers_arc.lock().await;
        peers_lock.remove(&info_hash);
        drop(peers_lock);

        if persistent {
            self.remove_update(info_hash).await;
            self.remove_shadow(info_hash).await;
        }

        if removed_torrent { self.update_stats(StatsEvent::Torrents, -1).await; }
        if remove_seeders != 0 { self.update_stats(StatsEvent::Seeds, remove_seeders).await; }
        if remove_leechers != 0 { self.update_stats(StatsEvent::Peers, remove_leechers).await; }
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>, persistent: bool) -> Result<(), ()>
    {
        for info_hash in hashes.iter() {
            let torrent_option = match timeout(Duration::from_secs(30), async move {
                let torrents_arc = self.map_torrents.clone();
                let torrents_lock = torrents_arc.lock().await;
                let torrent_option = torrents_lock.get(info_hash).cloned();
                drop(torrents_lock);
                torrent_option
            }).await {
                Ok(data) => { data }
                Err(_) => {
                    error!("[REMOVE_TORRENTS] Read Lock (torrents) request timed out!");
                    return Err(());
                }
            };

            if torrent_option.is_some() {
                self.remove_torrent(*info_hash, persistent).await;
            }
        }
        Ok(())
    }
}