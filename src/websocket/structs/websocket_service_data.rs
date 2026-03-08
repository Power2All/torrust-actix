use crate::config::structs::configuration::Configuration;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use std::sync::Arc;

pub struct WebSocketServiceData {
    pub tracker: Arc<TorrentTracker>,
    pub config: Arc<Configuration>,
    pub master_id: String,
}