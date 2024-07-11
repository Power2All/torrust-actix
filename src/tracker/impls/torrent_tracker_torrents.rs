use std::collections::HashMap;
use std::sync::Arc;
use log::{debug, info};
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

    pub async fn add_torrent(&self, info_hash: InfoHash, torrent_entry: TorrentEntry, persistent: bool)
    {
        let torrents_arc = self.torrents_sharding.clone();
        torrents_arc.insert(info_hash, torrent_entry.clone());
        if persistent {
            self.add_torrents_update(info_hash, torrent_entry.completed).await;
        }
    }

    pub async fn add_torrents(&self, torrents: HashMap<InfoHash, TorrentEntry>, persistent: bool)
    {
        let mut updates = HashMap::new();
        for (info_hash, torrent_entry) in torrents.iter() {
            debug!("[DEBUG] Calling add_torrent");
            self.add_torrent(*info_hash, torrent_entry.clone(), persistent).await;
            updates.insert(*info_hash, torrent_entry);
        }
    }

    pub async fn get_torrent(&self, info_hash: InfoHash) -> Option<TorrentEntry>
    {
        let torrents_arc = self.torrents_sharding.clone();

        torrents_arc.get(&info_hash)
    }

    pub async fn get_torrents(&self, hashes: Vec<InfoHash>) -> HashMap<InfoHash, Option<TorrentEntry>>
    {
        let mut return_torrents = HashMap::new();
        for info_hash in hashes.iter() {
            debug!("[DEBUG] Calling get_torrent");
            return_torrents.insert(*info_hash, self.get_torrent(*info_hash).await);
        }
        return_torrents
    }

    pub async fn remove_torrent(&self, info_hash: InfoHash, persistent: bool)
    {
        let torrents_arc = self.torrents_sharding.clone();
        torrents_arc.remove(info_hash);
        if persistent {
            self.remove_torrents_update(info_hash).await;
            self.remove_torrents_shadow(info_hash).await;
        }
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>, persistent: bool)
    {
        let torrents_arc = self.torrents_sharding.clone();
        for info_hash in hashes.iter() {
            if torrents_arc.get(info_hash).is_some() {
                debug!("[DEBUG] Calling remove_torrent");
                self.remove_torrent(*info_hash, persistent).await;
            }
        }
    }

    pub async fn get_torrents_stats(&self) -> (u64, u64, u64)
    {
        let torrents_arc = self.torrents_sharding.clone();
        let torrents = torrents_arc.len() as u64;
        let mut seeds = 0u64;
        let mut peers = 0u64;
        for shard in 0..255 {
            let shard = torrents_arc.get_shard(shard);
            for (_, torrent_entry) in shard {
                seeds += torrent_entry.seeds_count;
                peers += torrent_entry.peers_count;
            }
        }
        (torrents, seeds, peers)
    }
}
