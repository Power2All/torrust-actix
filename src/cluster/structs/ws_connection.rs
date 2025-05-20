use std::sync::Arc;
use reqwest_websocket::WebSocket;
use tokio::sync::Mutex;
use crate::cluster::structs::rx_data::RxData;
use crate::cluster::structs::tx_data::TxData;

#[derive(Debug)]
pub struct WsConnection {
    pub server_id: Option<String>,
    pub(crate) watcher: tokio::sync::watch::Receiver<bool>,
    pub websocket: Arc<Option<Mutex<WebSocket>>>
}