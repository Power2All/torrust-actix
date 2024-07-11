use std::collections::BTreeMap;
use std::sync::atomic::AtomicI64;
use crossbeam_skiplist::SkipMap;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

#[derive(Default)]
pub struct TorrentSharding {
    pub length: AtomicI64,
    pub shards: SkipMap<u8, BTreeMap<InfoHash, TorrentEntry>>
}