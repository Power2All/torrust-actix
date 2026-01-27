use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use std::sync::Arc;

#[derive(Debug)]
pub struct ApiServiceData {
    pub torrent_tracker: Arc<TorrentTracker>,
    pub api_trackers_config: Arc<ApiTrackersConfig>
}
