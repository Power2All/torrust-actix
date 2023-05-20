use chrono::Utc;
use log::debug;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};

use crate::tracker::TorrentTracker;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StatsEvent {
    Torrents,
    TorrentsUpdates,
    TorrentsShadow,
    Users,
    UsersUpdates,
    UsersShadow,
    TimestampSave,
    TimestampTimeout,
    TimestampConsole,
    TimestampKeysTimeout,
    MaintenanceMode,
    Seeds,
    Peers,
    Completed,
    Whitelist,
    Blacklist,
    Key,
    Tcp4ConnectionsHandled,
    Tcp4ApiHandled,
    Tcp4AnnouncesHandled,
    Tcp4ScrapesHandled,
    Tcp6ConnectionsHandled,
    Tcp6ApiHandled,
    Tcp6AnnouncesHandled,
    Tcp6ScrapesHandled,
    Udp4ConnectionsHandled,
    Udp4AnnouncesHandled,
    Udp4ScrapesHandled,
    Udp6ConnectionsHandled,
    Udp6AnnouncesHandled,
    Udp6ScrapesHandled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Stats {
    pub started: i64,
    pub timestamp_run_save: i64,
    pub timestamp_run_timeout: i64,
    pub timestamp_run_console: i64,
    pub timestamp_run_keys_timeout: i64,
    pub torrents: i64,
    pub torrents_updates: i64,
    pub torrents_shadow: i64,
    pub users: i64,
    pub users_updates: i64,
    pub users_shadow: i64,
    pub maintenance_mode: i64,
    pub seeds: i64,
    pub peers: i64,
    pub completed: i64,
    pub whitelist_enabled: bool,
    pub whitelist: i64,
    pub blacklist_enabled: bool,
    pub blacklist: i64,
    pub keys_enabled: bool,
    pub keys: i64,
    pub tcp4_connections_handled: i64,
    pub tcp4_api_handled: i64,
    pub tcp4_announces_handled: i64,
    pub tcp4_scrapes_handled: i64,
    pub tcp6_connections_handled: i64,
    pub tcp6_api_handled: i64,
    pub tcp6_announces_handled: i64,
    pub tcp6_scrapes_handled: i64,
    pub udp4_connections_handled: i64,
    pub udp4_announces_handled: i64,
    pub udp4_scrapes_handled: i64,
    pub udp6_connections_handled: i64,
    pub udp6_announces_handled: i64,
    pub udp6_scrapes_handled: i64,
}

impl TorrentTracker {
    pub fn channel_stats_init(&self)
    {
        let (_channel_left, channel_right) = self.stats_channel.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let mut stats: Stats = Stats {
                started: Utc::now().timestamp(),
                timestamp_run_save: 0,
                timestamp_run_timeout: 0,
                timestamp_run_console: 0,
                timestamp_run_keys_timeout: 0,
                torrents: 0,
                torrents_updates: 0,
                torrents_shadow: 0,
                users: 0,
                users_updates: 0,
                users_shadow: 0,
                maintenance_mode: 0,
                seeds: 0,
                peers: 0,
                completed: 0,
                whitelist_enabled: config.whitelist,
                whitelist: 0,
                blacklist_enabled: config.blacklist,
                blacklist: 0,
                keys_enabled: config.keys,
                keys: 0,
                tcp4_connections_handled: 0,
                tcp4_api_handled: 0,
                tcp4_announces_handled: 0,
                tcp4_scrapes_handled: 0,
                tcp6_connections_handled: 0,
                tcp6_api_handled: 0,
                tcp6_announces_handled: 0,
                tcp6_scrapes_handled: 0,
                udp4_connections_handled: 0,
                udp4_announces_handled: 0,
                udp4_scrapes_handled: 0,
                udp6_connections_handled: 0,
                udp6_announces_handled: 0,
                udp6_scrapes_handled: 0,
            };

            loop {
                match serde_json::from_str::<Value>(&channel_right.recv().unwrap()) {
                    Ok(data) => {
                        // debug!("Received: {:#?}", data);

                        // Main handler and interact with action.
                        match data["action"].as_str().unwrap() {
                            "get" => {
                                channel_right.send(json!({"action": "get", "data": stats}).to_string()).unwrap();
                            }
                            "set" => {
                                let event: StatsEvent = serde_json::from_value::<StatsEvent>(data["data"]["event"].clone()).unwrap();
                                let value: i64 = serde_json::from_value::<i64>(data["data"]["value"].clone()).unwrap();
                                match event {
                                    StatsEvent::Torrents => { stats.torrents = value; }
                                    StatsEvent::TorrentsUpdates => { stats.torrents_updates = value; }
                                    StatsEvent::TorrentsShadow => { stats.torrents_shadow = value; }
                                    StatsEvent::Users => { stats.users = value; }
                                    StatsEvent::UsersUpdates => { stats.users_updates = value; }
                                    StatsEvent::UsersShadow => { stats.users_shadow = value; }
                                    StatsEvent::TimestampSave => { stats.timestamp_run_save = value; }
                                    StatsEvent::TimestampTimeout => { stats.timestamp_run_timeout = value; }
                                    StatsEvent::TimestampConsole => { stats.timestamp_run_console = value; }
                                    StatsEvent::TimestampKeysTimeout => { stats.timestamp_run_keys_timeout = value; }
                                    StatsEvent::MaintenanceMode => { stats.maintenance_mode = value; }
                                    StatsEvent::Seeds => { stats.seeds = value; }
                                    StatsEvent::Peers => { stats.peers = value; }
                                    StatsEvent::Completed => { stats.completed = value; }
                                    StatsEvent::Whitelist => { stats.whitelist = value; }
                                    StatsEvent::Blacklist => { stats.blacklist = value; }
                                    StatsEvent::Key => { stats.keys = value; }
                                    StatsEvent::Tcp4ConnectionsHandled => { stats.tcp4_connections_handled = value; }
                                    StatsEvent::Tcp4ApiHandled => { stats.tcp4_api_handled = value; }
                                    StatsEvent::Tcp4AnnouncesHandled => { stats.tcp4_announces_handled = value; }
                                    StatsEvent::Tcp4ScrapesHandled => { stats.tcp4_scrapes_handled = value; }
                                    StatsEvent::Tcp6ConnectionsHandled => { stats.tcp6_connections_handled = value; }
                                    StatsEvent::Tcp6ApiHandled => { stats.tcp6_api_handled = value; }
                                    StatsEvent::Tcp6AnnouncesHandled => { stats.tcp6_announces_handled = value; }
                                    StatsEvent::Tcp6ScrapesHandled => { stats.tcp6_scrapes_handled = value; }
                                    StatsEvent::Udp4ConnectionsHandled => { stats.udp4_connections_handled = value; }
                                    StatsEvent::Udp4AnnouncesHandled => { stats.udp4_announces_handled = value; }
                                    StatsEvent::Udp4ScrapesHandled => { stats.udp4_scrapes_handled = value; }
                                    StatsEvent::Udp6ConnectionsHandled => { stats.udp6_connections_handled = value; }
                                    StatsEvent::Udp6AnnouncesHandled => { stats.udp6_announces_handled = value; }
                                    StatsEvent::Udp6ScrapesHandled => { stats.udp6_scrapes_handled = value; }
                                }
                                channel_right.send(json!({"action": "set", "data": stats}).to_string()).unwrap();
                            }
                            "update" => {
                                let event: StatsEvent = serde_json::from_value::<StatsEvent>(data["data"]["event"].clone()).unwrap();
                                let value: i64 = serde_json::from_value::<i64>(data["data"]["value"].clone()).unwrap();
                                match event {
                                    StatsEvent::Torrents => { stats.torrents += value; }
                                    StatsEvent::TorrentsUpdates => { stats.torrents_updates += value; }
                                    StatsEvent::TorrentsShadow => { stats.torrents_shadow += value; }
                                    StatsEvent::Users => { stats.users += value; }
                                    StatsEvent::UsersUpdates => { stats.users_updates += value; }
                                    StatsEvent::UsersShadow => { stats.users_shadow += value; }
                                    StatsEvent::TimestampSave => { stats.timestamp_run_save += value; }
                                    StatsEvent::TimestampTimeout => { stats.timestamp_run_timeout += value; }
                                    StatsEvent::TimestampConsole => { stats.timestamp_run_console += value; }
                                    StatsEvent::TimestampKeysTimeout => { stats.timestamp_run_keys_timeout += value; }
                                    StatsEvent::MaintenanceMode => { stats.maintenance_mode += value; }
                                    StatsEvent::Seeds => { stats.seeds += value; }
                                    StatsEvent::Peers => { stats.peers += value; }
                                    StatsEvent::Completed => { stats.completed += value; }
                                    StatsEvent::Whitelist => { stats.whitelist += value; }
                                    StatsEvent::Blacklist => { stats.blacklist += value; }
                                    StatsEvent::Key => { stats.keys += value; }
                                    StatsEvent::Tcp4ConnectionsHandled => { stats.tcp4_connections_handled += value; }
                                    StatsEvent::Tcp4ApiHandled => { stats.tcp4_api_handled += value; }
                                    StatsEvent::Tcp4AnnouncesHandled => { stats.tcp4_announces_handled += value; }
                                    StatsEvent::Tcp4ScrapesHandled => { stats.tcp4_scrapes_handled += value; }
                                    StatsEvent::Tcp6ConnectionsHandled => { stats.tcp6_connections_handled += value; }
                                    StatsEvent::Tcp6ApiHandled => { stats.tcp6_api_handled += value; }
                                    StatsEvent::Tcp6AnnouncesHandled => { stats.tcp6_announces_handled += value; }
                                    StatsEvent::Tcp6ScrapesHandled => { stats.tcp6_scrapes_handled += value; }
                                    StatsEvent::Udp4ConnectionsHandled => { stats.udp4_connections_handled += value; }
                                    StatsEvent::Udp4AnnouncesHandled => { stats.udp4_announces_handled += value; }
                                    StatsEvent::Udp4ScrapesHandled => { stats.udp4_scrapes_handled += value; }
                                    StatsEvent::Udp6ConnectionsHandled => { stats.udp6_connections_handled += value; }
                                    StatsEvent::Udp6AnnouncesHandled => { stats.udp6_announces_handled += value; }
                                    StatsEvent::Udp6ScrapesHandled => { stats.udp6_scrapes_handled += value; }
                                };
                                channel_right.send(json!({"action": "update", "data": stats}).to_string()).unwrap();
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

    pub async fn channel_stats_request(&self, action: &str, data: Value) -> (Value, Value)
    {
        let (channel_left, _channel_right) = self.stats_channel.clone();
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

    pub async fn get_stats(&self) -> Stats
    {
        let (_action, data) = self.channel_stats_request("get", json!({})).await;
        
        serde_json::from_value::<Stats>(data).unwrap()
    }

    pub async fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let (_action, data) = self.channel_stats_request("update", json!({
            "event": event,
            "value": value
        })).await;
        
        serde_json::from_value::<Stats>(data).unwrap()
    }

    pub async fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        let (_action, data) = self.channel_stats_request("set", json!({
            "event": event,
            "value": value
        })).await;
        
        serde_json::from_value::<Stats>(data).unwrap()
    }
}