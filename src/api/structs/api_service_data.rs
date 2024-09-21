use std::sync::Arc;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub struct ApiServiceData {
    pub(crate) torrent_tracker: Arc<TorrentTracker>,
    pub(crate) api_trackers_config: Arc<ApiTrackersConfig>
}
