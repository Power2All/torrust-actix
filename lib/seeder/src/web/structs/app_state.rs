use crate::config::structs::torrents_file::TorrentsFile;
use crate::stats::shared_stats::SharedStats;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{
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
}