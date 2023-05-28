use std::collections::HashMap;
use log::{debug, info};
use serde_json::{json, Value};
use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

impl TorrentTracker {
    pub fn channel_blacklist_init(&self)
    {
        let (_channel_left, channel_right) = self.blacklist_channel.clone();
        tokio::spawn(async move {
            let mut blacklist: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
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

    pub async fn channel_blacklist_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, _channel_right) = self.blacklist_channel.clone();
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

    pub async fn load_blacklists(&self)
    {
        if let Ok(blacklists) = self.sqlx.load_blacklist().await {
            let mut blacklist_count = 0i64;

            for info_hash in blacklists.iter() {
                self.add_blacklist(*info_hash, true).await;
                blacklist_count += 1;
            }

            info!("Loaded {} blacklists.", blacklist_count);
        }
    }

    pub async fn save_blacklists(&self) -> bool
    {
        let blacklist = self.get_blacklist().await;
        if self.sqlx.save_blacklist(blacklist).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn add_blacklist(&self, info_hash: InfoHash, on_load: bool)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        if on_load {
            blacklist_lock.insert(info_hash, 1i64);
        } else {
            blacklist_lock.insert(info_hash, 2i64);
        }
        drop(blacklist_lock);

        self.update_stats(StatsEvent::Blacklist, 1).await;
    }

    pub async fn get_blacklist(&self) -> Vec<InfoHash>
    {
        let mut return_list = vec![];

        let blacklist_arc = self.blacklist.clone();
        let blacklist_lock = blacklist_arc.read().await;
        for (info_hash, _) in blacklist_lock.iter() {
            return_list.push(*info_hash);
        }
        drop(blacklist_lock);

        return_list
    }

    pub async fn remove_flag_blacklist(&self, info_hash: InfoHash)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        if blacklist_lock.get(&info_hash).is_some() {
            blacklist_lock.insert(info_hash, 0i64);
        }
        let blacklists = blacklist_lock.clone();
        drop(blacklist_lock);

        let mut blacklist_count = 0i64;
        for (_, value) in blacklists.iter() {
            if value == &1i64 {
                blacklist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Blacklist, blacklist_count).await;
    }

    pub async fn remove_blacklist(&self, info_hash: InfoHash)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        blacklist_lock.remove(&info_hash);
        let blacklists = blacklist_lock.clone();
        drop(blacklist_lock);

        let mut blacklist_count = 0i64;
        for (_, value) in blacklists.iter() {
            if value == &1 {
                blacklist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Blacklist, blacklist_count).await;
    }

    pub async fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let blacklist_arc = self.blacklist.clone();
        let blacklist_lock = blacklist_arc.read().await;
        let blacklist = blacklist_lock.get(&info_hash).cloned();
        drop(blacklist_lock);

        if blacklist.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_blacklist(&self)
    {
        let blacklist_arc = self.blacklist.clone();
        let mut blacklist_lock = blacklist_arc.write().await;
        blacklist_lock.clear();
        drop(blacklist_lock);

        self.set_stats(StatsEvent::Blacklist, 0).await;
    }
}