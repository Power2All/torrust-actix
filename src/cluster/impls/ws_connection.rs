use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use log::{error, info};
use parking_lot::Mutex;
use reqwest::Client;
use reqwest_websocket::{Error, RequestBuilderExt, UpgradeResponse, WebSocket};
use crate::cluster::structs::rx_data::RxData;
use crate::cluster::structs::tx_data::TxData;
pub use crate::cluster::structs::ws_connection::WsConnection;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl WsConnection {
    pub async fn new(
        tracker: Arc<TorrentTracker>,
        rx: tokio::sync::watch::Receiver<bool>
    ) -> WsConnection {
        let address = SocketAddr::from_str(&tracker.config.tracker_config.cluster_server_address).unwrap();
        let socket = match address {
            SocketAddr::V4(ipv4) => {
                match tracker.config.tracker_config.cluster_ssl {
                    true => format!("wss://{}:{}/cluster?token={}", ipv4.ip(), ipv4.port(), tracker.config.tracker_config.api_key),
                    false => format!("ws://{}:{}/cluster?token={}", ipv4.ip(), ipv4.port(), tracker.config.tracker_config.api_key),
                }
            }
            SocketAddr::V6(ipv6) => {
                match tracker.config.tracker_config.cluster_ssl {
                    true => format!("wss://[{}]:{}/cluster?token={}", ipv6.ip(), ipv6.port(), tracker.config.tracker_config.api_key),
                    false => format!("ws://[{}]:{}/cluster?token={}", ipv6.ip(), ipv6.port(), tracker.config.tracker_config.api_key),
                }
            }
        };

        let mut ws_connection = WsConnection {
            server_id: None,
            socket: None,
            watcher: rx,
        };

        loop {
            info!("[BOOT] Trying to connect to: {} ...", &socket);
            let request_socket = Client::default().get(&socket).upgrade().send();
            match request_socket.await {
                Ok(socket_conn) => {
                    info!("[BOOT] Successfully connected to Websocket");
                    match socket_conn.into_websocket().await {
                        Ok(websocket) => {
                            ws_connection.socket = Some(Mutex::new(websocket));
                            break;
                        }
                        Err(error) => {
                            error!("[BOOT] Unable to connect to websocket: {}", error);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }
                Err(error) => {
                    error!("[BOOT] Unable to connect to websocket: {}", error);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        ws_connection
    }

    pub async fn listener(&self)
    {
        loop {
            if self.watcher.has_changed().unwrap_or(true) {
                break;
            }
            
            // Add your listener implementation here
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn send(&self, message: String) {
        if let Some(socket) = &self.socket {
            let mut locked_socket = socket.lock();
            // Implement send logic here
        }
    }
}