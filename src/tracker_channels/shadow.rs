use std::collections::HashMap;
use log::debug;
use serde_json::{json, Value};

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

impl TorrentTracker {
    pub fn channel_shadow_init(&self)
    {
        let (_channel_left, channel_right) = self.shadow_channel.clone();
        tokio::spawn(async move {
            let mut shadow: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let completed = serde_json::from_value::<i64>(data["data"]["completed"].clone()).unwrap();
                                let _ = shadow.insert(info_hash, completed);
                                channel_right.send(json!({
                                    "action": "add_single",
                                    "data": {},
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, completed) in hashes.iter() {
                                    let _ = shadow.insert(*info_hash, *completed);
                                }
                                channel_right.send(json!({
                                    "action": "add_multi",
                                    "data": {},
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = shadow.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "get_single",
                                    "data": torrent,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = shadow.get(info_hash);
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "get_multi",
                                    "data": return_data,
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match shadow.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "delete_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_multi" => {
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
                                    "action": "delete_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "shadow_count": shadow.len() as i64
                                }).to_string()).unwrap();
                            }
                            "shutdown" => {
                                channel_right.send(json!({"action": "shutdown", "data": {}}).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({"action": "error", "data": "unknown action"}).to_string()).unwrap();
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Received: {:#?}", error);
                        channel_right.send(json!({"action": "error", "data": error.to_string()}).to_string()).unwrap();
                    }
                }
            }
        });
    }

    pub async fn channel_shadow_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, _channel_right) = self.shadow_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub async fn add_shadow(&self, info_hash: InfoHash, completed: i64)
    {
        let shadow_arc = self.shadow.clone();
        let mut shadow_lock = shadow_arc.write().await;
        shadow_lock.insert(info_hash, completed);
        let shadow_count = shadow_lock.len();
        drop(shadow_lock);

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_shadow(&self, info_hash: InfoHash)
    {
        let shadow_arc = self.shadow.clone();
        let mut shadow_lock = shadow_arc.write().await;
        shadow_lock.remove(&info_hash);
        let shadow_count = shadow_lock.len();
        drop(shadow_lock);

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn remove_shadows(&self, hashes: Vec<InfoHash>)
    {
        let mut shadow_count = 0;

        for info_hash in hashes.iter() {
            let shadow_arc = self.shadow.clone();
            let mut shadow_lock = shadow_arc.write().await;
            shadow_lock.remove(info_hash);
            shadow_count = shadow_lock.len();
            drop(shadow_lock);
        }

        self.set_stats(StatsEvent::TorrentsShadow, shadow_count as i64).await;
    }

    pub async fn get_shadow(&self) -> HashMap<InfoHash, i64>
    {
        let shadow_arc = self.shadow.clone();
        let shadow_lock = shadow_arc.read().await;
        let shadow = shadow_lock.clone();
        drop(shadow_lock);

        shadow
    }

    pub async fn clear_shadow(&self)
    {
        let shadow_arc = self.shadow.clone();
        let mut shadow_lock = shadow_arc.write().await;
        shadow_lock.clear();
        drop(shadow_lock);

        self.set_stats(StatsEvent::TorrentsShadow, 0).await;
    }

    pub async fn transfer_updates_to_shadow(&self)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.write().await;
        let updates = updates_lock.clone();
        updates_lock.clear();
        drop(updates_lock);

        for (info_hash, completed) in updates.iter() {
            self.add_shadow(*info_hash, *completed).await;
        }

        self.set_stats(StatsEvent::TorrentsUpdates, 0).await;
    }
}