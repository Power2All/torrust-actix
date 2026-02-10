use crate::config::structs::webtorrent_trackers_config::WebTorrentTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use std::sync::Arc;

#[derive(Clone)]
pub struct WebTorrentServiceData {
    pub torrent_tracker: Arc<TorrentTracker>,
    pub webtorrent_config: Arc<WebTorrentTrackersConfig>,
}