use crate::app::enums::tab::Tab;
use crate::app::structs::torrent_entry::{TorrentEntry, demo_torrents};
use crate::app::types::DEFAULT_SPLIT_RATIO;

pub struct TorrustClientApp {
    pub split_ratio: f32,
    pub selected_torrent: Option<usize>,
    pub active_tab: Tab,
    pub torrents: Vec<TorrentEntry>,
}

impl Default for TorrustClientApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TorrustClientApp {
    pub fn new() -> Self {
        Self {
            split_ratio: DEFAULT_SPLIT_RATIO,
            selected_torrent: None,
            active_tab: Tab::default(),
            torrents: demo_torrents(),
        }
    }
}
