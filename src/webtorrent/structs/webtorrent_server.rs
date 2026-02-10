use crate::webtorrent::structs::webtorrent_service_data::WebTorrentServiceData;
use std::sync::Arc;

pub struct WebTorrentConnection {
    pub data: Arc<WebTorrentServiceData>,
    pub client_ip: Option<std::net::IpAddr>,
}