use std::net::SocketAddr;
use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use futures_util::{SinkExt, TryStreamExt};
use log::{error, info};
use parking_lot::RwLock;
use reqwest::Client;
use reqwest_websocket::{websocket, Error, Message, RequestBuilderExt, UpgradeResponse, WebSocket};
use serde_json::{json, Value};
use tokio::select;
use tokio::sync::Mutex;
use tokio::time::error::Elapsed;
use tokio::time::{sleep, timeout};
use uuid::Uuid;
use crate::cluster::structs::rx_data::RxData;
use crate::cluster::structs::tx_data::TxData;
pub use crate::cluster::structs::ws_connection::WsConnection;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl WsConnection {
    pub async fn new(
        tracker: Arc<TorrentTracker>,
        rx: tokio::sync::watch::Receiver<bool>
    ) -> WsConnection {
        let socket = Self::get_websocket_address(
            tracker.config.tracker_config.cluster_server_address.clone(),
            tracker.config.tracker_config.cluster_ssl.clone(),
            tracker.config.tracker_config.api_key.clone()
        );
        
        // Initialize the server connection info object
        let mut ws_connection = WsConnection { server_id: None, watcher: rx };
        
        // Setting up a connection to the server and keep looping it until succeeded
        loop {
            let websocket = Arc::new(Mutex::new(loop {
                select! {
                    websocket_result = Self::connect_to_websocket(&socket, tracker.config.tracker_config.cluster_timeout) => {
                        match websocket_result {
                            Ok(websocket) => {
                                break websocket;
                            }
                            Err(_) => {
                                sleep(Duration::from_secs(1)).await;
                                continue;
                            }
                        }
                    }
                    changed = ws_connection.watcher.changed() => {
                        return ws_connection;
                    }
                }
            }));

            // Let's validate the connection if usable
            select! {
                id = Self::validate_connection(websocket.clone(), tracker.clone()) => {
                    match id {
                        Ok(id_returned) => {
                            ws_connection.server_id = Some(id_returned);
                            break;
                        }
                        Err(_) => {}
                    }
                }
                changed = ws_connection.watcher.changed() => {
                    return ws_connection;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await
        }
        
        ws_connection
    }

    pub fn get_websocket_address(address: String, ssl: bool, api_key: String) -> String
    {
        let address = SocketAddr::from_str(&address).unwrap();
        match address {
            SocketAddr::V4(ipv4) => {
                match ssl {
                    true => format!("wss://{}:{}/cluster?token={}", ipv4.ip(), ipv4.port(), api_key),
                    false => format!("ws://{}:{}/cluster?token={}", ipv4.ip(), ipv4.port(), api_key),
                }
            }
            SocketAddr::V6(ipv6) => {
                match ssl {
                    true => format!("wss://[{}]:{}/cluster?token={}", ipv6.ip(), ipv6.port(), api_key),
                    false => format!("ws://[{}]:{}/cluster?token={}", ipv6.ip(), ipv6.port(), api_key),
                }
            }
        }
    }
    
    pub async fn connect_to_websocket(socket: &String, timeout_int: u64) -> Result<WebSocket, ()>
    {
        info!("[BOOT] Trying to connect to leader: {} ...", socket);
        match timeout(Duration::from_secs(timeout_int), Client::default().get(socket).upgrade().send()).await {
            Ok(socket_response) => {
                match socket_response {
                    Ok(socket_conn) => {
                        match socket_conn.into_websocket().await {
                            Ok(websocket) => {
                                info!("[BOOT] Websocket connected to: {} ...", socket);
                                return Ok(websocket);
                            }
                            Err(_) => {
                                error!("[BOOT] Unable to connect to: {} ...", socket);
                            }
                        }
                    }
                    Err(_) => {
                        error!("[BOOT] Unable to connect to: {} ...", socket);
                    }
                }
            }
            Err(_) => {
                error!("[BOOT] Unable to connect to: {} ...", socket);
            }
        }
        Err(())
    }
    
    pub async fn validate_connection(socket: Arc<Mutex<WebSocket>>, tracker: Arc<TorrentTracker>) -> Result<String, ()>
    {
        info!("[BOOT] Validating connection...");
        let request = json!({
            "version": env!("CARGO_PKG_VERSION"),
            "server_id": tracker.server_id.clone()
        });
        
        // Create the message outside the mutex lock
        let message = Message::Text(serde_json::to_string(&request).unwrap());
        
        let socket_lock = socket.clone();
        let mut socket_guard = socket_lock.lock().await;
        match socket_guard.deref_mut().send(message.clone()).await {
            Ok(_) => {}
            Err(_) => { return Err(()); }
        }
        
        // Wait for a response with a new mutex lock
        match socket_guard.deref_mut().try_next().await {
            Ok(Some(Message::Text(response))) => {
                let response = match serde_json::from_str::<Value>(&response) {
                    Ok(data_response) => { data_response }
                    Err(error) => {
                        error!("[BOOT] Failed to receive leader ID. Message: {}", error);
                        return Err(());
                    }
                };

                if response.get("version").is_some() && response.get("server_id").is_some() {
                    let response_server_version = response.get("version").unwrap().as_str().unwrap();
                    let response_server_id = response.get("server_id").unwrap().as_str().unwrap();
                    if response_server_version != env!("CARGO_PKG_VERSION") {
                        error!("Invalid server version. Expected: {}, Received: {}", response_server_version, env!("CARGO_PKG_VERSION"));
                        return Err(());
                    }
                    match Uuid::parse_str(&response_server_id) {
                        Ok(_) => {
                            info!("[BOOT] Leader with ID {} and version {} established", response_server_id.to_uppercase(), response_server_version);
                            return Ok(response_server_id.to_uppercase());
                        }
                        Err(_) => {
                            error!("Invalid server ID format received: {}", response_server_id.to_uppercase());
                            return Err(());
                        }
                    }
                }
            }
            _ => {}
        }
        
        Err(())
    }
}