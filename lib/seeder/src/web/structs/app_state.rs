use crate::config::structs::torrents_file::TorrentsFile;
use crate::stats::shared_stats::SharedStats;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{
    broadcast,
    watch,
    Mutex,
    RwLock,
};

pub type SessionStore = Arc<Mutex<HashMap<String, std::time::Instant>>>;

pub struct AppState {
    pub yaml_path: PathBuf,
    pub shared_file: Arc<RwLock<TorrentsFile>>,
    pub stats: SharedStats,
    pub reload_tx: watch::Sender<()>,
    pub web_password: Option<String>,
    pub sessions: SessionStore,
    /// Broadcast channel — new log lines are sent here for WebSocket clients.
    pub log_tx: broadcast::Sender<String>,
    /// Ring buffer of the last 10 000 log lines (std Mutex so it can be used in sync log::Log).
    pub log_buffer: Arc<std::sync::Mutex<VecDeque<String>>>,
}
