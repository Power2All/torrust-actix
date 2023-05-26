use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use log::{debug, info};
use serde_json::{json, Value};
use crate::common::InfoHash;
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

impl TorrentTracker {
    pub fn channel_whitelist_blacklist_keys_init(&self)
    {
        let (_channel_left, channel_right) = self.whitelist_blacklist_keys_channel.clone();
        tokio::spawn(async move {
            let mut whitelist: HashMap<InfoHash, i64> = HashMap::new();
            let mut blacklist: HashMap<InfoHash, i64> = HashMap::new();
            let mut keys: HashMap<InfoHash, i64> = HashMap::new();

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            /* == Whitelist == */
                            "whitelist_add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let code = serde_json::from_value::<i64>(data["data"]["code"].clone()).unwrap();
                                let _ = whitelist.insert(info_hash, code);
                                channel_right.send(json!({
                                    "action": "whitelist_add_single",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, code) in hashes.iter() {
                                    let _ = whitelist.insert(*info_hash, *code);
                                }
                                channel_right.send(json!({
                                    "action": "whitelist_add_multi",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = whitelist.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "whitelist_get_single",
                                    "data": torrent,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = whitelist.get(info_hash);
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "whitelist_get_multi",
                                    "data": return_data,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_get_all" => {
                                let torrents = whitelist.clone();
                                channel_right.send(json!({
                                    "action": "whitelist_get_all",
                                    "data": torrents,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_remove_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match whitelist.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "whitelist_remove_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_remove_multi" => {
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
                                    "action": "whitelist_remove_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "whitelist_clear" => {
                                whitelist.clear();
                                channel_right.send(json!({
                                    "action": "whitelist_clear",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }

                            /* == Blacklist == */
                            "blacklist_add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let code = serde_json::from_value::<i64>(data["data"]["code"].clone()).unwrap();
                                let _ = blacklist.insert(info_hash, code);
                                channel_right.send(json!({
                                    "action": "blacklist_add_single",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, code) in hashes.iter() {
                                    let _ = blacklist.insert(*info_hash, *code);
                                }
                                channel_right.send(json!({
                                    "action": "blacklist_add_multi",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = blacklist.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "blacklist_get_single",
                                    "data": torrent,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = blacklist.get(info_hash);
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "blacklist_get_multi",
                                    "data": return_data,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_get_all" => {
                                let torrents = blacklist.clone();
                                channel_right.send(json!({
                                    "action": "blacklist_get_all",
                                    "data": torrents,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_remove_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match blacklist.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "blacklist_remove_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_remove_multi" => {
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                let mut removed: u64 = 0;
                                for info_hash in hashes.iter() {
                                    match blacklist.remove(info_hash) {
                                        None => {}
                                        Some(_) => {
                                            removed += 1;
                                        }
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "blacklist_remove_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "blacklist_clear" => {
                                blacklist.clear();
                                channel_right.send(json!({
                                    "action": "blacklist_clear",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }

                            /* == Keys == */
                            "keys_add_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let timeout = serde_json::from_value::<i64>(data["data"]["timeout"].clone()).unwrap();
                                let _ = keys.insert(info_hash, timeout);
                                channel_right.send(json!({
                                    "action": "keys_add_single",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_add_multi" => {
                                let hashes = serde_json::from_value::<Vec<(InfoHash, i64)>>(data["data"]["hashes"].clone()).unwrap();
                                for (info_hash, timeout) in hashes.iter() {
                                    let _ = keys.insert(*info_hash, *timeout);
                                }
                                channel_right.send(json!({
                                    "action": "keys_add_multi",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_get_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let torrent = keys.get(&info_hash);
                                channel_right.send(json!({
                                    "action": "keys_get_single",
                                    "data": torrent,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_get_multi" => {
                                let mut return_data = Vec::new();
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                for info_hash in hashes.iter() {
                                    let torrent = keys.get(info_hash);
                                    return_data.push((info_hash, torrent));
                                }
                                channel_right.send(json!({
                                    "action": "keys_get_multi",
                                    "data": return_data,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_get_all" => {
                                let torrents = keys.clone();
                                channel_right.send(json!({
                                    "action": "keys_get_all",
                                    "data": torrents,
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_remove_single" => {
                                let info_hash = serde_json::from_value::<InfoHash>(data["data"]["info_hash"].clone()).unwrap();
                                let removed = match keys.remove(&info_hash) {
                                    None => { false }
                                    Some(_) => { true }
                                };
                                channel_right.send(json!({
                                    "action": "keys_remove_single",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_remove_multi" => {
                                let hashes = serde_json::from_value::<Vec<InfoHash>>(data["data"]["hashes"].clone()).unwrap();
                                let mut removed: u64 = 0;
                                for info_hash in hashes.iter() {
                                    match keys.remove(info_hash) {
                                        None => {}
                                        Some(_) => {
                                            removed += 1;
                                        }
                                    }
                                }
                                channel_right.send(json!({
                                    "action": "keys_remove_multi",
                                    "data": {
                                        "removed": removed
                                    },
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }
                            "keys_clear" => {
                                keys.clear();
                                channel_right.send(json!({
                                    "action": "keys_clear",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                            }

                            "shutdown" => {
                                channel_right.send(json!({
                                    "action": "shutdown",
                                    "data": {},
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
                                break;
                            }
                            _ => {
                                channel_right.send(json!({
                                    "action": "error",
                                    "data": "unknown action",
                                    "whitelist_count": wbk_count(&whitelist),
                                    "blacklist_count": wbk_count(&blacklist),
                                    "keys_count": wbk_count(&keys)
                                }).to_string()).unwrap();
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

    pub async fn channel_whitelist_blacklist_keys_request(&self, action: &str, data: Value) -> (Value, Value, Value, Value, Value)
    {
        let (channel_left, _channel_right) = self.whitelist_blacklist_keys_channel.clone();
        // Build the data with a action and data separated.
        let request_data = json!({
            "action": action,
            "data": data
        });
        channel_left.send(request_data.to_string()).unwrap();
        let response = channel_left.recv().unwrap();
        let response_data: Value = serde_json::from_str(&response).unwrap();
        (
            response_data["action"].clone(),
            response_data["data"].clone(),
            response_data["whitelist_count"].clone(),
            response_data["blacklist_count"].clone(),
            response_data["keys_count"].clone()
        )
    }

    /* == Whitelist == */
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
        let mut code = 2i64;
        if on_load { code = 1i64; }
        let (_action, _data, whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "whitelist_add_single",
            json!({
                "info_hash": info_hash,
                "code": code
            })
        ).await;
        self.set_stats(StatsEvent::Whitelist, serde_json::from_value::<i64>(whitelist_count).unwrap()).await;
    }

    pub async fn get_whitelist(&self) -> HashMap<InfoHash, i64>
    {
        let (_action, data, whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "whitelist_get_all",
            json!({})
        ).await;
        self.set_stats(StatsEvent::Whitelist, serde_json::from_value::<i64>(whitelist_count).unwrap()).await;
        serde_json::from_value::<HashMap<InfoHash, i64>>(data).unwrap()
    }

    pub async fn remove_flag_whitelist(&self, info_hash: InfoHash)
    {
        let (_action, data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "whitelist_add_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let whitelist_option = serde_json::from_value::<Option<i64>>(data).unwrap();
        if whitelist_option.is_some() {
            let (_action, _data, whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
                "",
                json!({
                    "info_hash": info_hash,
                    "code": 0i64
                })
            ).await;
            self.set_stats(StatsEvent::Whitelist, serde_json::from_value::<i64>(whitelist_count).unwrap()).await;
        }
    }

    pub async fn remove_whitelist(&self, info_hash: InfoHash)
    {
        let (_action, data, whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "whitelist_delete_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        if serde_json::from_value::<bool>(data["removed"].clone()).unwrap() {
            self.set_stats(StatsEvent::Whitelist, serde_json::from_value::<i64>(whitelist_count).unwrap()).await;
        }
    }

    pub async fn check_whitelist(&self, info_hash: InfoHash) -> bool
    {
        let (_action, data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "whitelist_get_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        if serde_json::from_value::<Option<i64>>(data["removed"].clone()).unwrap().is_some() {
            return true;
        }
        false
    }

    pub async fn clear_whitelist(&self)
    {
        let (_action, _data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "whitelist_clear",
            json!({})
        ).await;
        self.set_stats(StatsEvent::Whitelist, 0).await;
    }

    /* == Blacklist == */
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
        if self.sqlx.save_blacklist(Vec::from_iter(blacklist.keys().cloned())).await.is_ok() {
            for (info_hash, value) in blacklist.iter() {
                if value == &0 {
                    self.remove_blacklist(*info_hash).await;
                }
                if value == &2 {
                    self.add_blacklist(*info_hash, true).await;
                }
            }
            return true;
        }
        false
    }

    pub async fn add_blacklist(&self, info_hash: InfoHash, on_load: bool)
    {
        let mut code = 2i64;
        if on_load { code = 1i64; }
        let (_action, _data, _whitelist_count, blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "blacklist_add_single",
            json!({
                "info_hash": info_hash,
                "code": code
            })
        ).await;
        self.set_stats(StatsEvent::Blacklist, serde_json::from_value::<i64>(blacklist_count).unwrap()).await;
    }

    pub async fn get_blacklist(&self) -> HashMap<InfoHash, i64>
    {
        let (_action, data, _whitelist_count, blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "blacklist_get_all",
            json!({})
        ).await;
        self.set_stats(StatsEvent::Blacklist, serde_json::from_value::<i64>(blacklist_count).unwrap()).await;
        serde_json::from_value::<HashMap<InfoHash, i64>>(data).unwrap()
    }

    pub async fn remove_flag_blacklist(&self, info_hash: InfoHash)
    {
        let (_action, data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "blacklist_add_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let blacklist_option = serde_json::from_value::<Option<i64>>(data).unwrap();
        if blacklist_option.is_some() {
            let (_action, _data, _whitelist_count, blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
                "",
                json!({
                    "info_hash": info_hash,
                    "code": 0i64
                })
            ).await;
            self.set_stats(StatsEvent::Blacklist, serde_json::from_value::<i64>(blacklist_count).unwrap()).await;
        }
    }

    pub async fn remove_blacklist(&self, info_hash: InfoHash)
    {
        let (_action, data, _whitelist_count, blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "blacklist_delete_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        if serde_json::from_value::<bool>(data["removed"].clone()).unwrap() {
            self.set_stats(StatsEvent::Blacklist, serde_json::from_value::<i64>(blacklist_count).unwrap()).await;
        }
    }

    pub async fn check_blacklist(&self, info_hash: InfoHash) -> bool
    {
        let (_action, data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "blacklist_get_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        if serde_json::from_value::<Option<i64>>(data["removed"].clone()).unwrap().is_some() {
            return true;
        }
        false
    }

    pub async fn clear_blacklist(&self)
    {
        let (_action, _data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "blacklist_clear",
            json!({})
        ).await;
        self.set_stats(StatsEvent::Blacklist, 0).await;
    }

    /* == Keys == */
    pub async fn load_keys(&self)
    {
        if let Ok(keys) = self.sqlx.load_keys().await {
            let mut keys_count = 0i64;

            for (info_hash, timeout) in keys.iter() {
                self.add_key(*info_hash, *timeout).await;
                keys_count += 1;
            }

            info!("Loaded {} keys.", keys_count);
        }
    }

    pub async fn save_keys(&self) -> bool
    {
        let keys = self.get_keys().await;
        let mut keys_parse = Vec::new();
        for (info_hash, code) in keys.iter() {
            keys_parse.push((*info_hash, *code));
        }
        if self.sqlx.save_keys(keys_parse).await.is_ok() {
            return true;
        }
        false
    }

    pub async fn add_key(&self, info_hash: InfoHash, timeout: i64)
    {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_unix = timestamp.as_secs() as i64 + timeout;
        let (_action, _data, _whitelist_count, _blacklist_count, keys_count) = self.channel_whitelist_blacklist_keys_request(
            "keys_add_single",
            json!({
                "info_hash": info_hash,
                "timeout": timeout_unix
            })
        ).await;
        self.set_stats(StatsEvent::Key, serde_json::from_value::<i64>(keys_count).unwrap()).await;
    }

    pub async fn get_keys(&self) -> HashMap<InfoHash, i64>
    {
        let (_action, data, _whitelist_count, _blacklist_count, keys_count) = self.channel_whitelist_blacklist_keys_request(
            "keys_get_all",
            json!({})
        ).await;
        self.set_stats(StatsEvent::Key, serde_json::from_value::<i64>(keys_count).unwrap()).await;
        serde_json::from_value::<HashMap<InfoHash, i64>>(data).unwrap()
    }

    pub async fn remove_flag_key(&self, info_hash: InfoHash)
    {
        let (_action, data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "keys_add_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        let key_option = serde_json::from_value::<Option<i64>>(data).unwrap();
        if key_option.is_some() {
            let (_action, _data, _whitelist_count, _blacklist_count, keys_count) = self.channel_whitelist_blacklist_keys_request(
                "",
                json!({
                    "info_hash": info_hash,
                    "code": 0i64
                })
            ).await;
            self.set_stats(StatsEvent::Key, serde_json::from_value::<i64>(keys_count).unwrap()).await;
        }
    }

    pub async fn remove_key(&self, info_hash: InfoHash)
    {
        let (_action, data, _whitelist_count, _blacklist_count, keys_count) = self.channel_whitelist_blacklist_keys_request(
            "keys_delete_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        if serde_json::from_value::<bool>(data["removed"].clone()).unwrap() {
            self.set_stats(StatsEvent::Key, serde_json::from_value::<i64>(keys_count).unwrap()).await;
        }
    }

    pub async fn check_key(&self, info_hash: InfoHash) -> bool
    {
        let (_action, data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "keys_get_single",
            json!({
                "info_hash": info_hash
            })
        ).await;
        if serde_json::from_value::<Option<i64>>(data["removed"].clone()).unwrap().is_some() {
            return true;
        }
        false
    }

    pub async fn clear_keys(&self)
    {
        let (_action, _data, _whitelist_count, _blacklist_count, _keys_count) = self.channel_whitelist_blacklist_keys_request(
            "keys_clear",
            json!({})
        ).await;
        self.set_stats(StatsEvent::Key, 0).await;
    }

    pub async fn clean_keys(&self)
    {
        let mut keys_index = vec![];
        for (info_hash, timeout) in self.get_keys().await.iter() {
            if *timeout != 0 {
                let time = SystemTime::from(Utc.timestamp_opt(*timeout, 0).unwrap());
                match time.duration_since(SystemTime::now()) {
                    Ok(_) => {}
                    Err(_) => { self.remove_key(*info_hash).await; }
                }
            }
            keys_index.push((*info_hash, *timeout));
        }
    }
}

pub fn wbk_count(torrents: &HashMap<InfoHash, i64>) -> u64
{
    let mut count = 0u64;
    for (_info_hash, code) in torrents.iter() {
        if *code == 1i64 {
            count += 1;
        }
    }
    count
}