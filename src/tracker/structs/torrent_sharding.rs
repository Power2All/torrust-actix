use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

#[derive(Debug, Default)]
pub struct TorrentSharding {
    pub shard_bag: HashMap<u8, Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>>,
}