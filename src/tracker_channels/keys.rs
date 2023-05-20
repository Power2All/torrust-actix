use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use log::{debug, info};
use serde_json::{json, Value};

use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

impl TorrentTracker {
    pub fn channel_keys_init(&self)
    {
        let (_channel_left, channel_right) = self.keys_channel.clone();
        tokio::spawn(async move {
            let mut keys: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
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

    pub async fn channel_keys_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, _channel_right) = self.keys_channel.clone();
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

    pub async fn load_keys(&self)
    {
        if let Ok(keys) = self.sqlx.load_keys().await {
            let mut keys_count = 0i64;

            for (hash, timeout) in keys.iter() {
                self.add_key_raw(*hash, *timeout).await;
                keys_count += 1;
            }

            info!("Loaded {} keys.", keys_count);
        }
    }

    pub async fn save_keys(&self) -> bool
    {
        let keys = self.get_keys().await;
        if self.sqlx.save_keys(keys).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn add_key(&self, hash: InfoHash, timeout: i64)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_unix = timestamp.as_secs() as i64 + timeout;
        keys_lock.insert(hash, timeout_unix);
        drop(keys_lock);

        self.update_stats(StatsEvent::Key, 1).await;
    }

    pub async fn add_key_raw(&self, hash: InfoHash, timeout: i64)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        let time = SystemTime::from(Utc.timestamp_opt(timeout, 0).unwrap());
        match time.duration_since(SystemTime::now()) {
            Ok(_) => {
                keys_lock.insert(hash, timeout);
            }
            Err(_) => {
                drop(keys_lock);
                return;
            }
        }
        drop(keys_lock);

        self.update_stats(StatsEvent::Key, 1).await;
    }

    pub async fn get_keys(&self) -> Vec<(InfoHash, i64)>
    {
        let keys_arc = self.keys.clone();
        let keys_lock = keys_arc.read().await;
        let keys = keys_lock.clone();
        drop(keys_lock);

        let mut return_list = vec![];
        for (hash, timeout) in keys.iter() {
            return_list.push((*hash, *timeout));
        }

        return_list
    }

    pub async fn remove_key(&self, hash: InfoHash)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        keys_lock.remove(&hash);
        let key_count = keys_lock.len();
        drop(keys_lock);

        self.set_stats(StatsEvent::Key, key_count as i64).await;
    }

    pub async fn check_key(&self, hash: InfoHash) -> bool
    {
        let keys_arc = self.keys.clone();
        let keys_lock = keys_arc.read().await;
        let key = keys_lock.get(&hash).cloned();
        drop(keys_lock);

        if key.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_keys(&self)
    {
        let keys_arc = self.keys.clone();
        let mut keys_lock = keys_arc.write().await;
        keys_lock.clear();
        drop(keys_lock);

        self.set_stats(StatsEvent::Key, 0).await;
    }

    pub async fn clean_keys(&self)
    {
        let keys_arc = self.keys.clone();
        let keys_lock = keys_arc.read().await;
        let keys = keys_lock.clone();
        drop(keys_lock);

        let mut keys_index = vec![];
        for (hash, timeout) in keys.iter() {
            keys_index.push((*hash, *timeout));
        }

        for (hash, timeout) in keys_index.iter() {
            if *timeout != 0 {
                let time = SystemTime::from(Utc.timestamp_opt(*timeout, 0).unwrap());
                match time.duration_since(SystemTime::now()) {
                    Ok(_) => {}
                    Err(_) => {
                        self.remove_key(*hash).await;
                    }
                }
            }
        }
    }
}