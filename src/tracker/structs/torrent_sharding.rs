use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct TorrentSharding {
    pub shards: [Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>; 256],
}