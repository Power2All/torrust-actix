use std::sync::Arc;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

#[derive(Debug)]
pub struct HttpServiceData {
    pub torrent_tracker: Arc<TorrentTracker>,
    pub http_trackers_config: Arc<HttpTrackersConfig>
}