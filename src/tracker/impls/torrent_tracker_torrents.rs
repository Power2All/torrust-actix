use std::collections::BTreeMap;
use std::sync::Arc;
use log::info;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>)
    {
        if let Ok((torrents, completes)) = self.sqlx.load_torrents(tracker.clone()).await {
            info!("Loaded {} torrents with {} completes.", torrents, completes);
            self.set_stats(StatsEvent::Completed, completes as i64).await;
        }
    }

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry) -> TorrentEntry
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).await.unwrap();
        let torrent = shard.get_or_insert(info_hash, torrent_entry).value().clone();
        torrent
    }

    pub async fn add_torrents(&self, hashes: BTreeMap<InfoHash, TorrentEntry>) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let mut returned_data = BTreeMap::new();
        for (info_hash, torrent_entry) in hashes.iter() {
            returned_data.insert(*info_hash, self.add_torrent(*info_hash, torrent_entry.clone()).await);
        }
        returned_data
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).await.unwrap();
        shard.get(&info_hash).map(|torrent| torrent.value().clone())    }

    pub async fn get_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, self.get_torrent(*info_hash).await);
        }
        returned_data
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let shard = self.torrents_sharding.clone().get_shard(info_hash.0[0]).await.unwrap();
        shard.remove(&info_hash).map(|data| data.value().clone())
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>) -> BTreeMap<InfoHash, Option<TorrentEntry>>
    {
        let mut returned_data = BTreeMap::new();
        for info_hash in hashes.iter() {
            returned_data.insert(*info_hash, self.remove_torrent(*info_hash).await);
        }
        returned_data
    }
}
