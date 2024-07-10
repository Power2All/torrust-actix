use std::collections::HashMap;
use std::ops::Add;
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
        let torrents_arc = self.torrents.clone();
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
        let torrents_arc = self.torrents.clone();

        let torrent = torrents_arc.get(&info_hash).map(|torrent| torrent.value().clone());
        torrent
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
        let torrents_arc = self.torrents.clone();
        torrents_arc.remove(&info_hash);
        if persistent {
            self.remove_torrents_update(info_hash).await;
            self.remove_torrents_shadow(info_hash).await;
        }
    }

    pub async fn remove_torrents(&self, hashes: Vec<InfoHash>, persistent: bool)
    {
        let torrents_arc = self.torrents.clone();
        for info_hash in hashes.iter() {
            if torrents_arc.get(info_hash).is_some() {
                debug!("[DEBUG] Calling remove_torrent");
                self.remove_torrent(*info_hash, persistent).await;
            }
        }
    }

    pub async fn get_torrents_chunk(&self, skip: u64, amount: u64) -> HashMap<InfoHash, i64>
    {
        let torrents_arc = self.torrents.clone();
        let mut torrents_return: HashMap<InfoHash, i64> = HashMap::new();
        let mut current_count: u64 = 0;
        let mut handled_count: u64 = 0;
        for torrent in torrents_arc.iter() {
            if current_count < skip {
                current_count = current_count.add(1);
                continue;
            }
            if handled_count >= amount { break; }
            torrents_return.insert(*torrent.key(), torrent.value().completed);
            current_count = current_count.add(1);
            handled_count = handled_count.add(1);
        }
        torrents_return
    }

    pub async fn get_torrents_stats(&self) -> (u64, u64, u64)
    {
        let torrents_arc = self.torrents.clone();
        let torrents = torrents_arc.len() as u64;
        let mut seeds = 0u64;
        let mut peers = 0u64;
        let mut start: usize = 0;
        let mut count: u64 = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        loop {
            if start > torrents_arc.len() {
                break;
            }
            for torrent in torrents_arc.iter().skip(start) {
                count += 1;
                seeds += torrent.value().seeds_count;
                peers += torrent.value().peers_count;
                if count == size as u64 {
                    break;
                }
            }
            start += size;
        }
        (torrents, seeds, peers)
    }
}
