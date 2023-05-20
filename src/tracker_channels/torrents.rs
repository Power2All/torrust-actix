use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use log::{debug, info};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};

use crate::common::{InfoHash, PeerId, TorrentPeer};
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

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

impl TorrentTracker {
    pub fn channel_torrents_init(&self)
    {
        let (channel_left, channel_right) = self.torrents_channel.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let mut torrents: BTreeMap<InfoHash, TorrentEntryItem> = BTreeMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let entry = serde_json::from_value::<TorrentEntryItem>(data["data"]["entry"].clone()).unwrap();
                                let _ = torrents.insert(info_hash, entry);
                                channel_right.send(json!({
                                    "action": "add_single",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, TorrentEntryItem)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, torrent_entry) in hashes.iter() {
                                    let _ = torrents.insert(info_hash.clone(), torrent_entry.clone());
                                }
                                channel_right.send(json!({
                                    "action": "add_multi",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = torrents.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "get_single",
                                    "data": torrent,
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi" => {
                                let mut torrentslist: Vec<(InfoHash, Option<TorrentEntry>)> = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for &info_hash in hashes.iter() {
                                    let torrent = match torrents.get(&info_hash) {
                                        None => { None }
                                        Some(torrent_entry) => {
                                            Some(TorrentEntry {
                                                peers: Default::default(),
                                                completed: torrent_entry.completed,
                                                seeders: torrent_entry.seeders,
                                                leechers: torrent_entry.leechers,
                                            })
                                        }
                                    };
                                    torrentslist.push((info_hash.clone(), torrent));
                                }
                                channel_right.send(json!({
                                    "action": "get_multi",
                                    "data": torrentslist,
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi_chunks" => {
                                let mut torrentslist: Vec<(InfoHash, i64)> = Vec::new();
                                let skip: u64 = serde_json::from_value::<u64>(data["data"]["skip"].clone()).unwrap();
                                let amount: u64 = serde_json::from_value::<u64>(data["data"]["amount"].clone()).unwrap();
                                let mut current_count: u64 = 0;
                                let mut handled_count: u64 = 0;
                                for (info_hash, entry) in torrents.iter() {
                                    if current_count < skip {
                                        current_count = current_count.add(1);
                                        continue;
                                    }
                                    if handled_count >= amount {
                                        break;
                                    }
                                    torrentslist.push((*info_hash, entry.completed));
                                    current_count = current_count.add(1);
                                    handled_count = handled_count.add(1);
                                }
                                channel_right.send(json!({
                                    "action": "get_multi_chunks",
                                    "data": torrentslist,
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match torrents.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "delete_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_multi" => {
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                let mut removed: u64 = 0;
                                for info_hash in hashes.iter() {
                                    match torrents.remove(info_hash) {
                                        None => {}
                                        Some(_) => {
                                            removed += 1;
                                        }
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "delete_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shutdown" => {
                                channel_right.send(json!({
                                    "action": "shutdown",
                                    "data": {},
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({
                                    "action": "error",
                                    "data": "unknown action",
                                    "torrent_count": torrents.len() as i64
                                }).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({
                            "action": "error",
                            "data": error.to_string(),
                            "torrent_count": torrents.len() as i64
                        }).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_torrents_request(&self, action: &str, data: Value) -> (Value, Value, Value)
    {
        let (channel_left, channel_right) = self.torrents_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone(), response_data["torrent_count"].clone())
    }

    pub async fn load_torrents(&self)
    {
        if let Ok(torrents) = self.sqlx.load_torrents().await {
            let mut torrent_count = 0i64;
            let mut completed_count = 0i64;

            for (info_hash, completed) in torrents.iter() {
                self.add_torrent(*info_hash, TorrentEntryItem {
                    completed: *completed,
                    seeders: 0,
                    leechers: 0,
                }, false).await;
                torrent_count += 1;
                completed_count += *completed;
            }

            info!("Loaded {} torrents with {} completes.", torrent_count, completed_count);
            self.update_stats(StatsEvent::Completed, completed_count).await;
        }
    }

    pub async fn save_torrents(&self) -> bool
    {
        let shadow = self.get_shadow().await;
        if self.sqlx.save_torrents(shadow).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntryItem, persistent: bool)
    {
        let (action, data, torrent_count) = self.channel_torrents_request(
            "add_single",
            json!({
                "info_hash": info_hash.clone(),
                "entry": torrent_entry.clone()
            })
        ).await;
        let torrents_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        self.update_stats(StatsEvent::Torrents, torrents_count).await;
        if persistent { self.add_update(info_hash, torrent_entry.completed).await; }
    }

    pub async fn add_torrents(&self, torrents: Vec<(InfoHash, TorrentEntryItem)>, persistent: bool)
    {
        let (_action, _data, torrent_count) = self.channel_torrents_request(
            "add_multi",
            json!({
                "hashes": torrents.clone()
            })
        ).await;
        let torrents_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        self.update_stats(StatsEvent::Torrents, torrents_count).await;
        if persistent {
            let mut updates = Vec::new();
            for (info_hash, torrent_entry) in torrents.iter() {
                updates.push((info_hash.clone(), torrent_entry.completed.clone()));
            }
            // self.add_updates(updates).await;
        }
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let (action, data, torrent_count) = self.channel_torrents_request(
            "get_single",
            json!({
                "info_hash": info_hash.clone()
            })
        ).await;
        let torrent_data = serde_json::from_value::<Option<TorrentEntryItem>>(data).unwrap();
        let torrent = match torrent_data {
            None => { None }
            Some(data) => {
                let peers_arc = self.map_peers.clone();
                let peers_lock = peers_arc.read().await;
                let peers = match peers_lock.get(&info_hash).cloned() {
                    None => { BTreeMap::new() }
                    Some(data) => { data }
                };
                drop(peers_lock);
                Some(TorrentEntry {
                    peers,
                    completed: data.completed,
                    seeders: data.seeders,
                    leechers: data.leechers,
                })
            }
        };
        torrent
    }

    pub async fn get_torrents(&self, hashes: Vec<InfoHash>) -> HashMap<InfoHash, Option<TorrentEntry>>
    {
        let mut return_torrents = HashMap::new();
        let (action, data, torrent_count) = self.channel_torrents_request(
            "get_multi",
            json!({
                "hashes": hashes.clone()
            })
        ).await;
        let torrents_data = serde_json::from_value::<Vec<(InfoHash, Option<TorrentEntry>)>>(data).unwrap();
        for (info_hash, torrent_entry) in torrents_data.iter() {
            return_torrents.insert(info_hash.clone(), match torrent_entry {
                None => { None }
                Some(data) => {
                    let peers_arc = self.map_peers.clone();
                    let peers_lock = peers_arc.read().await;
                    let peers = match peers_lock.get(info_hash).cloned() {
                        None => { BTreeMap::new() }
                        Some(data) => { data }
                    };
                    drop(peers_lock);
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
        let mut return_torrents = HashMap::new();
        let (action, data, torrent_count) = self.channel_torrents_request(
            "get_multi_chunks",
            json!({
                "skip": skip.clone(),
                "amount": amount.clone()
            })
        ).await;
        let torrents_data = serde_json::from_value::<Vec<(InfoHash, Option<TorrentEntryItem>)>>(data).unwrap();
        for (info_hash, torrent_entry_item) in torrents_data.iter() {
            match torrent_entry_item {
                None => {}
                Some(torrent_entry) => {
                    return_torrents.insert(info_hash.clone(), torrent_entry.completed.clone());
                }
            }
        }
        return_torrents
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool)
    {
        let mut removed_torrent = false;
        let mut remove_seeders = 0i64;
        let mut remove_leechers = 0i64;
        let (action, data, torrent_count) = self.channel_torrents_request(
            "delete_single",
            json!({
                "info_hash": info_hash.clone()
            })
        ).await;
        let torrents_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let torrents_deleted = serde_json::from_value::<bool>(data["removed"].clone()).unwrap();
        self.update_stats(StatsEvent::Torrents, torrents_count).await;
        self.update_stats(StatsEvent::Seeds, 0 - remove_seeders).await;
        self.update_stats(StatsEvent::Peers, 0 - remove_leechers).await;
        if persistent {
            // self.remove_update(info_hash).await;
            // self.remove_shadow(info_hash).await;
        }
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>, persistent: bool)
    {
        let (action, data, torrent_count) = self.channel_torrents_request(
            "delete_multi",
            json!({
                "hashes": hashes.clone()
            })
        ).await;
        let torrents_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let torrents_deleted = serde_json::from_value::<bool>(data["removed"].clone()).unwrap();
        self.set_stats(StatsEvent::Torrents, torrents_count).await;
        // self.set_stats(StatsEvent::Seeds, 0 - remove_seeders).await;
        // self.set_stats(StatsEvent::Peers, 0 - remove_leechers).await;
        if persistent {
            // self.remove_updates(hashes).await;
            // self.remove_shadows(hashes).await;
        }
    }
}