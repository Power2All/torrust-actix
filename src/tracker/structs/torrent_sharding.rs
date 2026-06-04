use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::types::ahash_map::AHashMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug)]
pub struct TorrentSharding {
    pub shards: [Arc<RwLock<AHashMap<InfoHash, TorrentEntry>>>; 256],
}