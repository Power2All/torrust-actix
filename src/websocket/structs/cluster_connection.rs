use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::websocket::structs::websocket_service_data::WebSocketServiceData;
use std::sync::Arc;

pub struct ClusterConnection {
    pub data: Arc<WebSocketServiceData>,
    pub authenticated: bool,
    pub slave_id: Option<String>,
    pub encoding: ClusterEncoding,
}