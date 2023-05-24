use std::collections::BTreeMap;
use log::{debug, info};
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
    pub fn channel_torrents_peers_init(&self)
    {
        let (_channel_left, channel_right) = self.torrents_peers_channel.clone();
        let _config = self.config.clone();
        tokio::spawn(async move {
            let mut torrents: BTreeMap<InfoHash, TorrentEntryItem> = BTreeMap::new();
            let mut peers: BTreeMap<InfoHash, BTreeMap<PeerId, TorrentPeer>> = BTreeMap::new();

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        match data["action"].as_str().unwrap() {
                            /* == Torrents == */
                            "torrent_add" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent_entry_item = serde_json::from_value::<TorrentEntryItem>(data["data"]["torrent_entry_item"].clone()).unwrap();
                                let _ = torrents.insert(info_hash, torrent_entry_item);
                                channel_right.send(json!({
                                    "action": "torrent_add",
                                    "data": {},
                                    "torrent_count": torrents.len(),
                                    "peer_count": peers_count(&peers)
                                }).to_string()).unwrap();
                            }
                            "torrent_remove" => {
                                let mut removed_torrent = false;
                                let mut removed_seed_count = 0u64;
                                let mut removed_peer_count = 0u64;
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = match torrents.get(&info_hash) {
                                    None => { None }
                                    Some(torrent_get) => { Some(torrent_get) }
                                };
                                let peers_list = match peers.get(&info_hash) {
                                    None => { None }
                                    Some(peers_get) => { Some(peers_get) }
                                };
                                if torrent.is_some() {
                                    torrents.remove(&info_hash);
                                    removed_torrent = true;
                                }
                                if peers_list.is_some() {
                                    let peers_list_unwrapped = peers_list.unwrap();
                                    for (peer_id, torrent_peer) in peers_list_unwrapped.iter() {
                                        if torrent_peer.left == NumberOfBytes(0) {
                                            removed_seed_count += 1;
                                        } else {
                                            removed_peer_count += 1;
                                        }
                                    }
                                    peers.remove(&info_hash);
                                }
                                channel_right.send(json!({
                                    "action": "torrent_remove",
                                    "data": {
                                        "removed_torrent": removed_torrent,
                                        "removed_seed_count": removed_seed_count,
                                        "removed_peer_count": removed_peer_count
                                    },
                                    "torrent_count": torrents.len(),
                                    "peer_count": peers_count(&peers)
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
                                    "torrent_count": torrents.len(),
                                    "peer_count": peers_count(&peers)
                                }).to_string()).unwrap();
                            }
                            "torrents_get_chunk" => {}

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
                                        peers.insert(info_hash, peers_list.clone());
                                        TorrentEntry {
                                            peers: peers_list.clone(),
                                            completed: data_torrent.completed.clone(),
                                            seeders: data_torrent.seeders.clone(),
                                            leechers: data_torrent.leechers.clone(),
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
                                    "torrent_count": torrents.len(),
                                    "peer_count": peers_count(&peers)
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
                                        let peer_option = peers_list.get(&peer_id);
                                        if peer_option.is_some() {
                                            let peer = peer_option.unwrap().clone();
                                            if peer.left == NumberOfBytes(0) {
                                                peers_list.remove(&peer_id);
                                                data_torrent.seeders -= 1;
                                                removed_seeder = true;
                                            } else {
                                                peers_list.remove(&peer_id);
                                                data_torrent.leechers -= 1;
                                                removed_leecher = true;
                                            }
                                        }
                                        torrents.insert(info_hash, data_torrent.clone());
                                        if peers_list.is_empty() {
                                            peers.remove(&info_hash);
                                        } else {
                                            peers.insert(info_hash, peers_list.clone());
                                        }
                                        TorrentEntry {
                                            peers: peers_list.clone(),
                                            completed: data_torrent.completed.clone(),
                                            seeders: data_torrent.seeders.clone(),
                                            leechers: data_torrent.leechers.clone()
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
                                    "torrent_count": torrents.len(),
                                    "peer_count": peers_count(&peers)
                                }).to_string()).unwrap();
                            }
                            "peer_get" => {}
                            "peers_get_chunk" => {}

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
                                    "torrent_count": torrents.len(),
                                    "peer_count": peers_count(&peers)
                                }).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({
                            "action": "error",
                            "data": error.to_string(),
                            "torrent_count": torrents.len(),
                            "peer_count": peers_count(&peers)
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
        let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        self.update_stats(StatsEvent::Torrents, torrent_count).await;
        if persistent {
            self.add_update(info_hash, torrent_entry_item.completed).await;
        }
    }

    pub async fn add_torrents(&self, torrents: Vec<(InfoHash, TorrentEntryItem)>, persistent: bool)
    {
        for (info_hash, torrent_entry_item) in torrents.iter() {
            let (_action, _data, torrent_count, peer_count) = self.channel_torrents_peers_request(
                "torrent_add",
                json!({
                    "info_hash": *info_hash,
                    "torrent_entry_item": *torrent_entry_item
                })
            ).await;
            let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            self.update_stats(StatsEvent::Torrents, torrent_count).await;
            if persistent {
                self.add_update(*info_hash, torrent_entry_item.completed).await;
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
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
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
            let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
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
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        serde_json::from_value::<Option<TorrentEntry>>(data).unwrap()
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
            let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            let torrent_entry = serde_json::from_value::<Option<TorrentEntry>>(data).unwrap();
            return_data.insert(*info_hash, torrent_entry);
        }
        return_data
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
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        let added_seeder = serde_json::from_value::<bool>(data["added_seeder"].clone()).unwrap();
        let added_leecher = serde_json::from_value::<bool>(data["added_leecher"].clone()).unwrap();
        let removed_seeder = serde_json::from_value::<bool>(data["removed_seeder"].clone()).unwrap();
        let removed_leecher = serde_json::from_value::<bool>(data["removed_leecher"].clone()).unwrap();
        let completed_applied = serde_json::from_value::<bool>(data["completed_applied"].clone()).unwrap();
        let torrent_entry_return = serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap();
        if persistent && completed { self.add_update(info_hash, torrent_entry_return.completed).await }
        if added_seeder  { self.update_stats(StatsEvent::Seeds, 1).await; }
        if added_leecher  { self.update_stats(StatsEvent::Peers, 1).await; }
        if removed_seeder  { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher  { self.update_stats(StatsEvent::Peers, -1).await; }
        if completed_applied { self.update_stats(StatsEvent::Completed, 1).await; }
        torrent_entry_return
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
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        let removed_seeder = serde_json::from_value::<bool>(data["removed_seeder"].clone()).unwrap();
        let removed_leecher = serde_json::from_value::<bool>(data["removed_leecher"].clone()).unwrap();
        let torrent_entry_return = serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap();
        if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
        if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
        torrent_entry_return
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
            let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
            let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
            let removed_seeder = serde_json::from_value::<bool>(data["removed_seeder"].clone()).unwrap();
            let removed_leecher = serde_json::from_value::<bool>(data["removed_leecher"].clone()).unwrap();
            let torrent_entry_return = serde_json::from_value::<TorrentEntry>(data["torrent_entry"].clone()).unwrap();
            if removed_seeder { self.update_stats(StatsEvent::Seeds, -1).await; }
            if removed_leecher { self.update_stats(StatsEvent::Peers, -1).await; }
            return_data.insert(*info_hash, torrent_entry_return);
        }
        return_data
    }

    pub async fn get_peer(&self, info_hash: InfoHash, peer_id: PeerId) -> Option<TorrentPeer>
    {
        let (_action, data, torrent_count, peer_count) = self.channel_torrents_peers_request(
            "peer_get",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let torrent_count = serde_json::from_value::<i64>(torrent_count).unwrap();
        let peer_count = serde_json::from_value::<i64>(peer_count).unwrap();
        serde_json::from_value::<Option<TorrentPeer>>(data).unwrap()
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