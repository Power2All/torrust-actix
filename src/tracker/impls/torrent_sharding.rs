use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use crossbeam_skiplist::SkipMap;
use log::info;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;

impl TorrentSharding {
    pub fn default() -> TorrentSharding
    {
        let output = SkipMap::new();
        for count in 0u8..255u8 {
            info!("{:#?}", count);
            output.insert(count, BTreeMap::new());
        }
        output.insert(255u8, BTreeMap::new());
        TorrentSharding {
            length: Default::default(),
            shards: output
        }
    }

    pub fn get(&self, info_hash: &InfoHash) -> Option<TorrentEntry> {
        info!("{:#?}", &info_hash.0[0]);
        let binding = self.shards.get(&info_hash.0[0]).unwrap();
        let shard = binding.value();
        shard.get(info_hash).cloned()
    }

    pub fn get_shard(&self, shard: u8) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let binding = self.shards.get(&shard).unwrap();
        binding.value().clone()
    }

    pub fn insert(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) {
        let mut shard = self.shards.get(&info_hash.0[0]).unwrap().value().clone();
        shard.insert(info_hash, torrent_entry);
        self.shards.insert(info_hash.0[0], shard);
        self.length.fetch_add(1i64, Ordering::SeqCst);
    }

    pub fn remove(&self, info_hash: InfoHash) -> Option<TorrentEntry> {
        let mut shard = self.shards.get(&info_hash.0[0]).unwrap().value().clone();
        let removed = shard.remove(&info_hash);
        if removed.is_some() { self.length.fetch_sub(1i64, Ordering::SeqCst); }
        self.shards.insert(info_hash.0[0], shard);
        removed
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize
    {
        self.length.load(Ordering::SeqCst) as usize
    }
}