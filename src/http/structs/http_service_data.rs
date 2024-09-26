use std::sync::Arc;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub struct HttpServiceData {
    pub(crate) torrent_tracker: Arc<TorrentTracker>,
    pub(crate) http_trackers_config: Arc<HttpTrackersConfig>
}