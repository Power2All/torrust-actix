use std::collections::HashMap;
use log::debug;
use serde_json::{json, Value};
use crate::common::InfoHash;
use crate::tracker::{StatsEvent, TorrentTracker};

impl TorrentTracker {
    pub fn channel_updates_init(&self)
    {
        let (channel_left, channel_right) = self.updates_channel.clone();
        tokio::spawn(async move {
            let mut updates: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let completed = serde_json::from_value::<i64>(data["data"]["completed"].clone()).unwrap();
                                let _ = updates.insert(info_hash, completed);
                                channel_right.send(json!({
                                    "action": "add_single",
                                    "data": {},
                                    "updates_count": updates.len() as i64
                                }).to_string()).unwrap();
                            }
                            "add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, completed) in hashes.iter() {
                                    let _ = updates.insert(info_hash.clone(), completed.clone());
                                }
                                channel_right.send(json!({
                                    "action": "add_multi",
                                    "data": {},
                                    "updates_count": updates.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = updates.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "get_single",
                                    "data": torrent,
                                    "updates_count": updates.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = match updates.get(info_hash) {
                                        None => { None }
                                        Some(data) => { Some(data) }
                                    };
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "get_multi",
                                    "data": return_data,
                                    "updates_count": updates.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match updates.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "delete_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "updates_count": updates.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_multi" => {
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
                                    "action": "delete_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "updates_count": updates.len() as i64
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

    pub async fn channel_updates_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.updates_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&*response).unwrap();
        (response_data["action"].clone(), response_data["data"].clone())
    }

    pub async fn add_update(&self, info_hash: InfoHash, completed: i64)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.write().await;
        updates_lock.insert(info_hash, completed);
        let update_count = updates_lock.len();
        drop(updates_lock);

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn add_updates(&self, updates: HashMap<InfoHash, i64>)
    {
        let mut update_count = 0;

        for (info_hash, completed) in updates.iter() {
            let updates_arc = self.updates.clone();
            let mut updates_lock = updates_arc.write().await;
            updates_lock.insert(*info_hash, *completed);
            update_count = updates_lock.len();
            drop(updates_lock);
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn get_update(&self) -> HashMap<InfoHash, i64>
    {
        let updates_arc = self.updates.clone();
        let updates_lock = updates_arc.read().await;
        let updates = updates_lock.clone();
        drop(updates_lock);

        updates
    }

    pub async fn remove_update(&self, info_hash: InfoHash)
    {
        let updates_arc = self.updates.clone();
        let mut updates_lock = updates_arc.write().await;
        updates_lock.remove(&info_hash);
        let update_count = updates_lock.len();
        drop(updates_lock);

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }

    pub async fn remove_updates(&self, hashes: Vec<InfoHash>)
    {
        let mut update_count = 0;

        for info_hash in hashes.iter() {
            let updates_arc = self.updates.clone();
            let mut updates_lock = updates_arc.write().await;
            updates_lock.remove(info_hash);
            update_count = updates_lock.len();
            drop(updates_lock);
        }

        self.set_stats(StatsEvent::TorrentsUpdates, update_count as i64).await;
    }
}