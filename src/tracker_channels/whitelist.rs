use std::collections::HashMap;
use log::{debug, info};
use serde_json::{json, Value};
use crate::common::InfoHash;
use crate::tracker::{StatsEvent, TorrentTracker};

impl TorrentTracker {
    pub fn channel_whitelist_init(&self)
    {
        let (channel_left, channel_right) = self.whitelist_channel.clone();
        tokio::spawn(async move {
            let mut whitelist: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&*channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let code = serde_json::from_value::<i64>(data["data"]["code"].clone()).unwrap();
                                let _ = whitelist.insert(info_hash, code);
                                channel_right.send(json!({
                                    "action": "add_single",
                                    "data": {},
                                    "whitelist_count": whitelist.len() as i64
                                }).to_string()).unwrap();
                            }
                            "add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, code) in hashes.iter() {
                                    let _ = whitelist.insert(info_hash.clone(), code.clone());
                                }
                                channel_right.send(json!({
                                    "action": "add_multi",
                                    "data": {},
                                    "whitelist_count": whitelist.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = whitelist.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "get_single",
                                    "data": torrent,
                                    "whitelist_count": whitelist.len() as i64
                                }).to_string()).unwrap();
                            }
                            "get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = match whitelist.get(info_hash) {
                                        None => { None }
                                        Some(data) => { Some(data) }
                                    };
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "get_multi",
                                    "data": return_data,
                                    "whitelist_count": whitelist.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match whitelist.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "delete_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": whitelist.len() as i64
                                }).to_string()).unwrap();
                            }
                            "delete_multi" => {
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                let mut removed: u64 = 0;
                                for info_hash in hashes.iter() {
                                    match whitelist.remove(info_hash) {
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
                                    "whitelist_count": whitelist.len() as i64
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

    /* === Channel: Whitelist === */
    pub async fn channel_whitelist_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, channel_right) = self.whitelist_channel.clone();
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

    pub async fn load_whitelists(&self)
    {
        if let Ok(whitelists) = self.sqlx.load_whitelist().await {
            let mut whitelist_count = 0i64;

            for info_hash in whitelists.iter() {
                self.add_whitelist(*info_hash, true).await;
                whitelist_count += 1;
            }

            info!("Loaded {} whitelists.", whitelist_count);
        }
    }

    pub async fn save_whitelists(&self) -> bool
    {
        let whitelist = self.get_whitelist().await;
        if self.sqlx.save_whitelist(whitelist.clone()).await.is_ok() {
            for (info_hash, value) in whitelist.iter() {
                if value == &0 {
                    self.remove_whitelist(*info_hash).await;
                }
                if value == &2 {
                    self.add_whitelist(*info_hash, true).await;
                }
            }
            return true;
        }
        false
    }

    pub async fn add_whitelist(&self, info_hash: InfoHash, on_load: bool)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        if on_load {
            whitelist_lock.insert(info_hash, 1i64);
        } else {
            whitelist_lock.insert(info_hash, 2i64);
        }
        drop(whitelist_lock);

        self.update_stats(StatsEvent::Whitelist, 1).await;
    }

    pub async fn get_whitelist(&self) -> HashMap<InfoHash, i64>
    {
        let mut return_list = HashMap::new();

        let whitelist_arc = self.whitelist.clone();
        let whitelist_lock = whitelist_arc.read().await;
        for (info_hash, value) in whitelist_lock.iter() {
            return_list.insert(*info_hash, *value);
        }
        drop(whitelist_lock);

        return_list
    }

    pub async fn remove_flag_whitelist(&self, info_hash: InfoHash)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        if whitelist_lock.get(&info_hash).is_some() {
            whitelist_lock.insert(info_hash, 0i64);
        }
        let whitelists = whitelist_lock.clone();
        drop(whitelist_lock);

        let mut whitelist_count = 0i64;
        for (_, value) in whitelists.iter() {
            if value == &1i64 {
                whitelist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn remove_whitelist(&self, info_hash: InfoHash)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        whitelist_lock.remove(&info_hash);
        let whitelists = whitelist_lock.clone();
        drop(whitelist_lock);

        let mut whitelist_count = 0i64;
        for (_, value) in whitelists.iter() {
            if value == &1 {
                whitelist_count += 1;
            }
        }

        self.set_stats(StatsEvent::Whitelist, whitelist_count).await;
    }

    pub async fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let whitelist_arc = self.whitelist.clone();
        let whitelist_lock = whitelist_arc.read().await;
        let whitelist = whitelist_lock.get(&info_hash).cloned();
        drop(whitelist_lock);

        if whitelist.is_some() {
            return true;
        }

        false
    }

    pub async fn clear_whitelist(&self)
    {
        let whitelist_arc = self.whitelist.clone();
        let mut whitelist_lock = whitelist_arc.write().await;
        whitelist_lock.clear();
        drop(whitelist_lock);

        self.set_stats(StatsEvent::Whitelist, 0).await;
    }
}