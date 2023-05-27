use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use log::{debug, info};
use scc::ebr::Arc;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};

use crate::common::{InfoHash, NumberOfBytes, PeerId, TorrentPeer};
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
    pub fn channel_torrents_peers_init(&self)
    {
        let (_channel_left, channel_right) = self.torrents_peers_channel.clone();
        let _config = self.config.clone();
        tokio::spawn(async move {
            let mut torrents: BTreeMap<InfoHash, TorrentEntryItem> = BTreeMap::new();
            let mut torrents_count = 0u64;
            let mut peers: BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>> = BTreeMap::new();
            let mut peers_count = 0u64;

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        match data["action"].as_str().unwrap() {
                            /* == Torrents == */
                            "torrent_add" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent_entry_item = serde_json::from_value::<TorrentEntryItem>(data["data"]["torrent_entry_item"].clone()).unwrap();
                                torrents.insert(info_hash, torrent_entry_item);
                                torrents_count = torrents.len() as u64;
                                channel_right.send(json!({
                                    "action": "torrent_add",
                                    "data": {},
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                            "torrents_add" => {
                                let mut return_data = Vec::new();
                                let torrent_list = serde_json::from_value::<Vec<(InfoHash, TorrentEntryItem)>>(data["data"]["torrents"].clone()).unwrap();
                                for (info_hash, torrent_entry_item) in torrent_list.iter() {
                                    torrents.insert(info_hash.clone(), torrent_entry_item.clone());
                                    return_data.push((info_hash.clone(), torrent_entry_item.completed.clone()));
                                }
                                torrents_count = torrents.len() as u64;
                                channel_right.send(json!({
                                    "action": "torrents_add",
                                    "data": {
                                        "updates": return_data
                                    },
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                            "torrent_remove" => {
                                let mut removed_torrent = false;
                                let mut removed_seed_count = 0u64;
                                let mut removed_peer_count = 0u64;
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = torrents.get(&info_hash);
                                let peers_list = peers.get(&info_hash);
                                if torrent.is_some() {
                                    torrents.remove(&info_hash);
                                    torrents_count = torrents.len() as u64;
                                    removed_torrent = true;
                                }
                                if let Some(peers_list_unwrapped) = peers_list {
                                    for (_peer_id, torrent_peer) in peers_list_unwrapped.iter() {
                                        if torrent_peer.left == NumberOfBytes(0) {
                                            removed_seed_count += 1;
                                        } else {
                                            removed_peer_count += 1;
                                        }
                                    }
                                    peers.remove(&info_hash);
                                    peers_count -= 1;
                                }
                                channel_right.send(json!({
                                    "action": "torrent_remove",
                                    "data": {
                                        "removed_torrent": removed_torrent,
                                        "removed_seed_count": removed_seed_count,
                                        "removed_peer_count": removed_peer_count
                                    },
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                            "torrent_get" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent_entry = match torrents.get(&info_hash) {
                                    None => { None }
                                    Some(torrent_data) => {
                                        let peers_list = match peers.get(&info_hash) {
                                            None => { BTreeMap::new() }
                                            Some(peers_get) => {
                                                peers_get.clone()
                                            }
                                        };
                                        Some(TorrentEntry {
                                            peers: peers_list,
                                            completed: torrent_data.completed,
                                            seeders: torrent_data.seeders,
                                            leechers: torrent_data.leechers
                                        })
                                    }
                                };
                                channel_right.send(json!({
                                    "action": "torrent_get",
                                    "data": torrent_entry,
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                            "torrents_get_chunk" => {
                                let mut return_data = HashMap::new();
                                let skip = serde_json::from_value::<u64>(data["data"]["skip"].clone()).unwrap();
                                let mut current = skip;
                                let amount = serde_json::from_value::<u64>(data["data"]["amount"].clone()).unwrap();
                                for (info_hash, torrent_entry_item) in torrents.iter().skip(skip as usize) {
                                    let peers_list = match peers.get(info_hash) {
                                        None => { BTreeMap::new() }
                                        Some(peers_get) => {
                                            peers_get.clone()
                                        }
                                    };
                                    return_data.insert(*info_hash, TorrentEntry {
                                        peers: peers_list,
                                        completed: torrent_entry_item.completed,
                                        seeders: torrent_entry_item.seeders,
                                        leechers: torrent_entry_item.leechers
                                    });
                                    current += 1;
                                    if current >= skip + amount {
                                        break;
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "torrents_get_chunk",
                                    "data": return_data,
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }

                            /* == Peers == */
                            "peer_add" => {
                                let mut added_seeder = false;
                                let mut added_leecher = false;
                                let mut removed_seeder = false;
                                let mut removed_leecher = false;
                                let mut completed_applied = false;
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let peer_id = serde_json::from_value::<PeerId>(data["data"]["peer_id"].clone()).unwrap();
                                let torrent_peer = serde_json::from_value::<TorrentPeer>(data["data"]["torrent_peer"].clone()).unwrap();
                                let completed = serde_json::from_value::<bool>(data["data"]["completed"].clone()).unwrap();
                                let torrent = match torrents.get(&info_hash).cloned() {
                                    None => { TorrentEntry::new() }
                                    Some(mut data_torrent) => {
                                        let mut peers_list = match peers.get(&info_hash).cloned() {
                                            None => { BTreeMap::new() }
                                            Some(data_peers) => { data_peers }
                                        };
                                        match peers_list.get(&peer_id).cloned() {
                                            None => {
                                                if torrent_peer.left == NumberOfBytes(0) {
                                                    data_torrent.seeders += 1;
                                                    added_seeder = true;
                                                    if completed {
                                                        data_torrent.completed += 1;
                                                        completed_applied = true;
                                                    }
                                                } else {
                                                    data_torrent.leechers += 1;
                                                    added_leecher = true;
                                                }
                                                let _ = peers_list.insert(peer_id, torrent_peer);
                                                peers_count += 1;
                                            }
                                            Some(data_peer) => {
                                                if data_peer.left == NumberOfBytes(0) && torrent_peer.left != NumberOfBytes(0) {
                                                    data_torrent.seeders -= 1;
                                                    data_torrent.leechers += 1;
                                                    removed_seeder = true;
                                                    added_leecher = true;
                                                } else if data_peer.left != NumberOfBytes(0) && torrent_peer.left == NumberOfBytes(0) {
                                                    data_torrent.seeders += 1;
                                                    data_torrent.leechers -= 1;
                                                    added_seeder = true;
                                                    removed_leecher = true;
                                                    if completed {
                                                        data_torrent.completed += 1;
                                                        completed_applied = true;
                                                    }
                                                }
                                                let _ = peers_list.insert(peer_id, torrent_peer);
                                            }
                                        }
                                        torrents.insert(info_hash, data_torrent.clone());
                                        torrents_count = torrents.len() as u64;
                                        peers.insert(info_hash, peers_list.clone());
                                        TorrentEntry {
                                            peers: peers_list.clone(),
                                            completed: data_torrent.completed,
                                            seeders: data_torrent.seeders,
                                            leechers: data_torrent.leechers,
                                        }
                                    }
                                };
                                channel_right.send(json!({
                                    "action": "peer_add",
                                    "data": {
                                        "added_seeder": added_seeder,
                                        "added_leecher": added_leecher,
                                        "removed_seeder": removed_seeder,
                                        "removed_leecher": removed_leecher,
                                        "completed_applied": completed_applied,
                                        "torrent_entry": torrent
                                    },
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                            "peer_remove" => {
                                let mut removed_seeder = false;
                                let mut removed_leecher = false;
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let peer_id = serde_json::from_value::<PeerId>(data["data"]["peer_id"].clone()).unwrap();
                                let torrent = match torrents.get(&info_hash).cloned() {
                                    None => { TorrentEntry::new() }
                                    Some(mut data_torrent) => {
                                        let mut peers_list = match peers.get(&info_hash).cloned() {
                                            None => { BTreeMap::new() }
                                            Some(data_peers) => { data_peers }
                                        };
                                        if let Some(peer) = peers_list.get(&peer_id) {
                                            if peer.left == NumberOfBytes(0) {
                                                peers_list.remove(&peer_id);
                                                data_torrent.seeders -= 1;
                                                removed_seeder = true;
                                            } else {
                                                peers_list.remove(&peer_id);
                                                data_torrent.leechers -= 1;
                                                removed_leecher = true;
                                            }
                                            peers_count -= 1;
                                        }
                                        torrents.insert(info_hash, data_torrent.clone());
                                        if peers_list.is_empty() {
                                            peers.remove(&info_hash);
                                        } else {
                                            peers.insert(info_hash, peers_list.clone());
                                        }
                                        TorrentEntry {
                                            peers: peers_list.clone(),
                                            completed: data_torrent.completed,
                                            seeders: data_torrent.seeders,
                                            leechers: data_torrent.leechers
                                        }
                                    }
                                };
                                channel_right.send(json!({
                                    "action": "peer_remove",
                                    "data": {
                                        "removed_seeder": removed_seeder,
                                        "removed_leecher": removed_leecher,
                                        "torrent_entry": torrent
                                    },
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                            "peer_get" => {}
                            "peers_get_chunk" => {
                                let mut return_data = Vec::new();
                                let skip = serde_json::from_value::<u64>(data["data"]["skip"].clone()).unwrap();
                                let mut current = 0u64;
                                let amount = serde_json::from_value::<u64>(data["data"]["amount"].clone()).unwrap();
                                for (info_hash, peers_list) in peers.iter() {
                                    if current > skip + amount {
                                        break;
                                    }
                                    for (peer_id, torrent_peer) in peers_list.iter() {
                                        if current > skip + amount {
                                            break;
                                        }
                                        if current < skip {
                                            current += 1;
                                            continue;
                                        }
                                        return_data.push((info_hash.clone(), peer_id.clone(), torrent_peer.clone()));
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "peers_get_chunk",
                                    "data": return_data,
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }

                            "shutdown" => {
                                channel_right.send(json!({
                                    "action": "shutdown",
                                    "data": {},
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({
                                    "action": "error",
                                    "data": "unknown action",
                                    "torrent_count": torrents_count,
                                    "peer_count": peers_count
                                }).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({
                            "action": "error",
                            "data": error.to_string(),
                            "torrent_count": torrents_count,
                            "peer_count": peers_count
                        }).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_torrents_peers_request(&self, action: &str, data: Value) -> (Value, Value, Value, Value)
    {
        let (channel_left, _channel_right) = self.torrents_peers_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone(), response_data["torrent_count"].clone(), response_data["peer_count"].clone())
    }

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        let (torrents, completes) = match self.sqlx.load_torrents(tracker.clone()).await {
            Ok(data) => { data }
            Err(_) => { panic!("Unable to obtain data from database!"); }
        };
        info!("Loaded {} torrents with {} completes.", torrents, completes);
        self.update_stats(StatsEvent::Completed, completes as i64).await;
    }

    pub async fn save_torrents(&self) -> bool
    {
        let shadow = self.get_shadow().await;
        if self.sqlx.save_torrents(shadow).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry_item: TorrentEntryItem, persistent: bool)
    {
        let (_action, _data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "torrent_add",
            json!({
                "info_hash": info_hash,
                "torrent_entry_item": torrent_entry_item
            })
        ).await;
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        self.update_stats(StatsEvent::Torrents, torrent_count).await;
        if persistent {
            self.add_update(info_hash, torrent_entry_item.completed).await;
        }
    }

    pub async fn add_torrents(&self, torrents: Vec<(InfoHash, TorrentEntryItem)>, persistent: bool)
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "torrents_add",
            json!({
                "torrents": torrents
            })
        ).await;
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        let updates = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["updates"].clone()).unwrap();
        self.update_stats(StatsEvent::Torrents, torrent_count).await;
        if persistent {
            for (info_hash, completed) in updates.iter() {
                self.add_update(info_hash.clone(), completed.clone()).await;
            }
        }
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool)
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "torrent_remove",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        let removed_torrent = serde_json::from_value::<bool>(data["removed_torrent"].clone()).unwrap();
        let removed_seed_count = serde_json::from_value::<u64>(data["removed_seed_count"].clone()).unwrap();
        let removed_peer_count = serde_json::from_value::<u64>(data["removed_peer_count"].clone()).unwrap();
        if removed_torrent { self.update_stats(StatsEvent::Torrents, -1).await; }
        if removed_seed_count != 0 { self.update_stats(StatsEvent::Seeds, 0 - removed_seed_count as i64).await; }
        if removed_peer_count != 0 { self.update_stats(StatsEvent::Peers, 0 - removed_peer_count as i64).await; }
        if persistent { self.remove_update(info_hash).await; }
    }

    pub async fn remove_torrents(&self, torrents: Vec<InfoHash>, persistent: bool)
    {
        for info_hash in torrents.iter() {
            let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
                "torrent_remove",
                json!({
                    "info_hash": *info_hash
                })
            ).await;
            let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            let removed_torrent = serde_json::from_value::<bool>(data["removed_torrent"].clone()).unwrap();
            let removed_seed_count = serde_json::from_value::<u64>(data["removed_seed_count"].clone()).unwrap();
            let removed_peer_count = serde_json::from_value::<u64>(data["removed_peer_count"].clone()).unwrap();
            if removed_torrent { self.update_stats(StatsEvent::Torrents, -1).await; }
            if removed_seed_count != 0 { self.update_stats(StatsEvent::Seeds, 0 - removed_seed_count as i64).await; }
            if removed_peer_count != 0 { self.update_stats(StatsEvent::Peers, 0 - removed_peer_count as i64).await; }
            if persistent { self.remove_update(*info_hash).await; }
        }
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "torrent_get",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        serde_json::from_value::<Option<TorrentEntry>>(data.clone()).unwrap()
    }

    pub async fn get_torrents(&self, torrents: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut return_data = BTreeMap::new();
        for info_hash in torrents.iter() {
            let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
                "torrent_get",
                json!({
                    "info_hash": *info_hash
                })
            ).await;
            let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            return_data.insert(*info_hash, serde_json::from_value::<Option<TorrentEntry>>(data).unwrap());
        }
        return_data
    }

    pub async fn get_torrents_chunk(&self, skip: u64, amount: u64) -> HashMap<InfoHash, TorrentEntry>
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "torrents_get_chunk",
            json!({
                "skip": skip,
                "amount": amount
            })
        ).await;
        let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        serde_json::from_value::<HashMap<InfoHash, TorrentEntry>>(data).unwrap()
    }


    pub async fn add_peer(&self, info_hash: InfoHash, peer_id: PeerId, torrent_peer: TorrentPeer, completed: bool, persistent: bool) -> TorrentEntry
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "peer_add",
            json!({
                "info_hash": info_hash,
                "peer_id": peer_id,
                "torrent_peer": torrent_peer,
                "completed": completed
            })
        ).await;
        let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        let added_seeder = serde_json::from_value::<bool>(data["added_seeder"].clone()).unwrap();
        let added_leecher = serde_json::from_value::<bool>(data["added_leecher"].clone()).unwrap();
        let removed_seeder = serde_json::from_value::<bool>(data["removed_seeder"].clone()).unwrap();
        let removed_leecher = serde_json::from_value::<bool>(data["removed_leecher"].clone()).unwrap();
        let completed_applied = serde_json::from_value::<bool>(data["completed_applied"].clone()).unwrap();
        if persistent && completed { self.add_update(info_hash, serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap().completed).await }
        if added_seeder  { self.update_stats(StatsEvent::Seeds, 1).await; }
        if added_leecher  { self.update_stats(StatsEvent::Peers, 1).await; }
        if removed_seeder  { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher  { self.update_stats(StatsEvent::Peers, -1).await; }
        if completed_applied { self.update_stats(StatsEvent::Completed, 1).await; }
        serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap()
    }

    pub async fn remove_peer(&self, info_hash: InfoHash, peer_id: PeerId, _persistent: bool) -> TorrentEntry
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "peer_remove",
            json!({
                "info_hash": info_hash,
                "peer_id": peer_id
            })
        ).await;
        let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        let removed_seeder = serde_json::from_value::<bool>(data["removed_seeder"].clone()).unwrap();
        let removed_leecher = serde_json::from_value::<bool>(data["removed_leecher"].clone()).unwrap();
        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
        serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap()
    }

    pub async fn remove_peers(&self, torrents: Vec<(InfoHash, PeerId)>, _persistent: bool) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let mut return_data = BTreeMap::new();
        for (info_hash, peer_id) in torrents.iter() {
            let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
                "peer_remove",
                json!({
                "info_hash": *info_hash,
                "peer_id": *peer_id
            })
            ).await;
            let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            let removed_seeder = serde_json::from_value::<bool>(data["removed_seeder"].clone()).unwrap();
            let removed_leecher = serde_json::from_value::<bool>(data["removed_leecher"].clone()).unwrap();
            if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
            if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
            return_data.insert(*info_hash, serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap());
        }
        return_data
    }

    pub async fn get_peer(&self, info_hash: InfoHash, _peer_id: PeerId) -> Option<TorrentPeer>
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "peer_get",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        serde_json::from_value::<Option<TorrentPeer>>(data).unwrap()
    }

    pub async fn get_peers(&self, peers: Vec<(InfoHash, PeerId)>) -> BTreeMap<InfoHash, Option<TorrentPeer>>
    {
        let mut return_data = BTreeMap::new();
        for (info_hash, _peer_id) in peers.iter() {
            let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
                "peer_get",
                json!({
                "info_hash": info_hash
            })
            ).await;
            let _torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let _peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            return_data.insert(*info_hash, serde_json::from_value::<Option<TorrentPeer>>(data).unwrap());
        }
        return_data
    }

    pub async fn get_peers_chunk(&self, skip: u64, amount: u64) -> Vec<(InfoHash, PeerId, TorrentPeer)>
    {
        let (_action, data, _torrent_count, _peer_count) = self.channel_torrents_peers_request(
            "peers_get_chunk",
            json!({
                "skip": skip,
                "amount": amount
            })
        ).await;
        serde_json::from_value::<Vec<(InfoHash, PeerId, TorrentPeer)>>(data).unwrap()
    }

    pub async fn clean_peers(&self, peer_timeout: Duration)
    {
        let mut skip: usize = 0;
        let amount: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        let mut removed_peers = 0u64;
        loop {
            info!("[PEERS] Scanning peers {} to {}", skip, (skip + amount));
            let peers = self.get_peers_chunk(skip as u64, amount as u64).await;
            if !peers.is_empty() {
                for (info_hash, peer_id, torrent_peer) in peers.iter() {
                    if torrent_peer.updated.elapsed() > peer_timeout {
                        removed_peers += 1;
                        self.remove_peer(*info_hash, *peer_id, false).await;
                    }
                }
                skip += amount;
            } else {
                break;
            }
        }
        info!("[PEERS] Removed {} peers", removed_peers);
    }
}

pub fn peers_count(peers: &BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>>) -> u64
{
    let mut count = 0u64;
    for (_info_hash, peers) in peers.iter() {
        count += peers.len() as u64;
    }
    count
}