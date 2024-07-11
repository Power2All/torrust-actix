use std::collections::BTreeMap;
use std::process::exit;
use std::sync::atomic::Ordering;
use crossbeam_skiplist::map::Entry;
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
        match self.shards.get(&info_hash.0[0]) {
            None => { None }
            Some(shard) => {
                shard.value().get(info_hash).cloned()
            }
        }
    }

    pub fn get_shard(&self, shard: u8) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let binding = self.shards.get(&shard).unwrap();
        binding.value().clone()
    }

    pub fn insert(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) {
        match self.shards.get(&info_hash.0[0]) {
            None => {
                info!("Unable to get shard {}", &info_hash.0[0]);
            }
            Some(shard) => {
                let mut shard_unpacked = shard.value().clone();
                match shard_unpacked.get(&info_hash) {
                    None => {
                        shard_unpacked.insert(info_hash, torrent_entry);
                        self.length.fetch_add(1i64, Ordering::SeqCst);
                    }
                    Some(_) => {
                        shard_unpacked.insert(info_hash, torrent_entry);
                    }
                }
                info!("Inserting {} into shards", info_hash.0[0]);
                self.shards.insert(info_hash.0[0], shard_unpacked);
            }
        }
    }

    pub fn remove(&self, info_hash: InfoHash) -> Option<TorrentEntry> {
        match self.shards.get(&info_hash.0[0]) {
            None => {
                panic!("Unable to get shard {}", &info_hash.0[0]);
            }
            Some(shard) => {
                let mut shard_unpacked = shard.value().clone();
                let removed = shard_unpacked.remove(&info_hash);
                if removed.is_some() {
                    self.length.fetch_sub(1i64, Ordering::SeqCst);
                }
                self.shards.insert(info_hash.0[0], shard_unpacked);
                removed
            }
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize
    {
        self.length.load(Ordering::SeqCst) as usize
    }
}