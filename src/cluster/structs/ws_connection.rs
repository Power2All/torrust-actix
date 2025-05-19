use parking_lot::Mutex;
use reqwest_websocket::WebSocket;
use crate::cluster::structs::rx_data::RxData;
use crate::cluster::structs::tx_data::TxData;

#[derive(Debug)]
pub struct WsConnection {
    pub(crate) server_id: Option<String>,
    pub(crate) watcher: tokio::sync::watch::Receiver<bool>,
}