use std::collections::HashMap;
use log::debug;
use serde_json::{json, Value};
use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

impl TorrentTracker {
    pub fn channel_updates_shadow_init(&self)
    {
        let (_channel_left, channel_right) = self.updates_shadow_channel.clone();
        tokio::spawn(async move {
            let mut updates: HashMap<InfoHash, i64> = HashMap::new();
            let mut shadow: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            /* == Updates == */
                            "updates_add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let completed = serde_json::from_value::<i64>(data["data"]["completed"].clone()).unwrap();
                                let _ = updates.insert(info_hash, completed);
                                channel_right.send(json!({
                                    "action": "updates_add_single",
                                    "data": {},
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "updates_add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, completed) in hashes.iter() {
                                    let _ = updates.insert(*info_hash, *completed);
                                }
                                channel_right.send(json!({
                                    "action": "updates_add_multi",
                                    "data": {},
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "updates_get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = updates.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "updates_get_single",
                                    "data": torrent,
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "updates_get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = updates.get(info_hash);
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "updates_get_multi",
                                    "data": return_data,
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "updates_get_all" => {
                                let clear = serde_json::from_value::<bool>(data["data"]["clear"].clone()).unwrap();
                                let updates_clone = updates.clone();
                                if clear {
                                    updates.clear();
                                }
                                channel_right.send(json!({
                                    "action": "updates_get_all",
                                    "data": updates_clone,
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "updates_remove_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match updates.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "updates_remove_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "updates_remove_multi" => {
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                let mut removed: u64 = 0;
                                for info_hash in hashes.iter() {
                                    match updates.remove(info_hash) {
                                        None => {}
                                        Some(_) => {
                                            removed += 1;
                                        }
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "updates_remove_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }

                            /* == Shadow == */
                            "shadow_add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let completed = serde_json::from_value::<i64>(data["data"]["completed"].clone()).unwrap();
                                let _ = shadow.insert(info_hash, completed);
                                channel_right.send(json!({
                                    "action": "shadow_add_single",
                                    "data": {},
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, completed) in hashes.iter() {
                                    let _ = shadow.insert(*info_hash, *completed);
                                }
                                channel_right.send(json!({
                                    "action": "shadow_add_multi",
                                    "data": {},
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = updates.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "shadow_get_single",
                                    "data": torrent,
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = shadow.get(info_hash);
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "shadow_get_multi",
                                    "data": return_data,
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_get_all" => {
                                channel_right.send(json!({
                                    "action": "updates_get_single",
                                    "data": shadow.clone(),
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_remove_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match shadow.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "shadow_remove_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_remove_multi" => {
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                let mut removed: u64 = 0;
                                for info_hash in hashes.iter() {
                                    match shadow.remove(info_hash) {
                                        None => {}
                                        Some(_) => {
                                            removed += 1;
                                        }
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "shadow_remove_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shadow_clear" => {
                                shadow.clear();
                                channel_right.send(json!({
                                    "action": "shadow_clear",
                                    "data": {},
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }

                            "shutdown" => {
                                channel_right.send(json!({
                                    "action": "shutdown",
                                    "data": {},
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({
                                    "action": "error",
                                    "data": "unknown action",
                                    "updates_count": updates.len() as i64,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({
                            "action": "error",
                            "data": error.to_string(),
                            "updates_count": updates.len() as i64,
                            "shadow_count": shadow.len() as i64
                        }).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_updates_shadow_request(&self, action: &str, data: Value) -> (Value, Value, Value, Value)
    {
        let (channel_left, _channel_right) = self.updates_shadow_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone(), response_data["updates_count"].clone(), response_data["shadow_count"].clone())
    }

    pub async fn add_update(&self, info_hash: InfoHash, completed: i64)
    {
        let (_action, _data, updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "updates_add_single",
            json!({
                "info_hash": info_hash,
                "completed": completed
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsUpdates, serde_json::from_value::<i64>(updates_count).unwrap()).await;
    }

    pub async fn add_updates(&self, updates: HashMap<InfoHash, i64>)
    {
        let updates_vec: Vec<(InfoHash, i64)> = updates.into_iter().map(|(k, v)| (k, v)).collect();
        let (_action, _data, updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "updates_add_multi",
            json!({
                "hashes": updates_vec
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsUpdates, serde_json::from_value::<i64>(updates_count).unwrap()).await;
    }

    pub async fn get_update(&self) -> HashMap<InfoHash, i64>
    {
        let (_action, data, _updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "updates_get_all",
            json!({
                "clear": false
            })
        ).await;
        serde_json::from_value::<HashMap<InfoHash, i64>>(data).unwrap()
    }

    pub async fn remove_update(&self, info_hash: InfoHash)
    {
        let (_action, _data, updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "updates_remove_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsUpdates, serde_json::from_value::<i64>(updates_count).unwrap()).await;
    }

    pub async fn remove_updates(&self, updates: Vec<InfoHash>)
    {
        let (_action, _data, updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "updates_remove_multi",
            json!({
                "hashes": updates
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsUpdates, serde_json::from_value::<i64>(updates_count).unwrap()).await;
    }

    pub async fn add_shadow(&self, info_hash: InfoHash, completed: i64)
    {
        let (_action, _data, _updates_count, shadow_count) = self.channel_updates_shadow_request(
            "shadow_add_single",
            json!({
                "info_hash": info_hash,
                "completed": completed
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsShadow, serde_json::from_value::<i64>(shadow_count).unwrap()).await;
    }

    pub async fn add_shadows(&self, shadows: HashMap<InfoHash, i64>)
    {
        let shadows_vec: Vec<(InfoHash, i64)> = shadows.into_iter().map(|(k, v)| (k, v)).collect();
        let (_action, _data, _updates_count, shadow_count) = self.channel_updates_shadow_request(
            "shadow_add_multi",
            json!({
                "hashes": shadows_vec
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsShadow, serde_json::from_value::<i64>(shadow_count).unwrap()).await;
    }

    pub async fn get_shadow(&self) -> HashMap<InfoHash, i64>
    {
        let (_action, data, _updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "shadow_get_all",
            json!({})
        ).await;
        serde_json::from_value::<HashMap<InfoHash, i64>>(data).unwrap()
    }

    pub async fn remove_shadow(&self, info_hash: InfoHash)
    {
        let (_action, _data, _updates_count, shadow_count) = self.channel_updates_shadow_request(
            "shadow_remove_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsShadow, serde_json::from_value::<i64>(shadow_count).unwrap()).await;
    }

    pub async fn remove_shadows(&self, shadows: Vec<InfoHash>)
    {
        let (_action, _data, _updates_count, shadow_count) = self.channel_updates_shadow_request(
            "shadow_remove_multi",
            json!({
                "hashes": shadows
            })
        ).await;
        self.set_stats(StatsEvent::TorrentsShadow, serde_json::from_value::<i64>(shadow_count).unwrap()).await;
    }

    pub async fn clear_shadow(&self)
    {
        let (_action, _data, _updates_count, shadow_count) = self.channel_updates_shadow_request(
            "shadow_clear",
            json!({})
        ).await;
        self.set_stats(StatsEvent::TorrentsShadow, serde_json::from_value::<i64>(shadow_count).unwrap()).await;
    }

    pub async fn transfer_updates_to_shadow(&self)
    {
        let (_action, data, _updates_count, _shadow_count) = self.channel_updates_shadow_request(
            "updates_get_all",
            json!({
                "clear": true
            })
        ).await;
        let updates = serde_json::from_value::<HashMap<InfoHash, i64>>(data).unwrap();
        for (info_hash, completed) in updates.iter() {
            self.add_shadow(*info_hash, *completed).await;
        }
    }
}