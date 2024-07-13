use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use concat_arrays::concat_arrays;
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

    pub async fn add_torrent(torrent_tracker: Arc<TorrentTracker>, info_hash: InfoHash, torrent_entry: TorrentEntry, persistent: bool, threaded: bool)
    {
        let torrents_map = torrent_tracker.torrents_map.clone();
        if threaded {
            tokio::spawn(async move {
                torrents_map.insert(info_hash, TorrentEntry::default());
                if persistent {
                    torrent_tracker.add_torrents_update(info_hash, torrent_entry.completed.load(Ordering::SeqCst) as i64).await;
                }
            });
        } else {
            torrents_map.insert(info_hash, TorrentEntry::default());
            if persistent {
                torrent_tracker.add_torrents_update(info_hash, torrent_entry.completed.load(Ordering::SeqCst) as i64).await;
            }
        }
    }

    pub async fn get_torrent(&self, info_hash: &InfoHash) -> Option<TorrentEntry>
    {
        let torrents_map = self.torrents_map.clone();
        let return_data = match torrents_map.get(&info_hash) {
            None => { None }
            Some(torrent) => {
                Some(TorrentEntry {
                    seeds: AtomicU64::new(torrent.value().seeds.load(Ordering::SeqCst)),
                    peers: AtomicU64::new(torrent.value().peers.load(Ordering::SeqCst)),
                    completed: AtomicU64::new(torrent.value().completed.load(Ordering::SeqCst)),
                    updated: std::time::Instant::now()
                })
            }
        };
        return_data
    }

    pub async fn remove_torrent(&self, info_hash: &InfoHash, persistent: bool) -> bool
    {
        let torrents_map = self.torrents_map.clone();
        let seeds_map = self.seeds_map.clone();
        let peers_map = self.peers_map.clone();
        let start_range: [u8; 40] = concat_arrays!(info_hash.0, [0; 20]);
        let end_range: [u8; 40] = concat_arrays!(info_hash.0, [255; 20]);
        for seed in seeds_map.range(start_range..=end_range) {
            seeds_map.remove(seed.key());
        }
        for peer in peers_map.range(start_range..=end_range) {
            seeds_map.remove(peer.key());
        }
        let return_data = match torrents_map.remove(info_hash) {
            None => { false }
            Some(_) => { true }
        };
        return_data
    }

    pub async fn get_torrents_chunk(&self, skip: usize, amount: usize) -> HashMap<InfoHash, u64>
    {
        let torrents_map = self.torrents_map.clone();
        let mut torrents_return: HashMap<InfoHash, u64> = HashMap::new();
        let mut current_count: usize = 0;
        let mut handled_count: usize = 0;
        for torrent in torrents_map.iter().skip(skip) {
            if handled_count >= amount {
                break;
            }
            torrents_return.insert(*torrent.key(), torrent.value().completed.load(Ordering::SeqCst));
            current_count = current_count.add(1);
            handled_count = handled_count.add(1);
        }
        torrents_return
    }

    pub async fn get_torrents_stats(&self) -> (u64, u64, u64)
    {
        let torrents_map = self.torrents_map.clone();
        let seeds_map = self.seeds_map.clone();
        let peers_map = self.peers_map.clone();
        let mut seeds = 0u64;
        let mut peers = 0u64;
        let mut start: usize = 0;
        let mut torrents: usize = 0;
        let size: usize = self.config.cleanup_chunks.unwrap_or(100000) as usize;
        loop {
            if start > torrents_map.len() {
                break;
            }
            for torrent in torrents_map.iter().skip(start) {
                torrents += 1;
                let start_range: [u8; 40] = concat_arrays!(torrent.key().0, [0; 20]);
                let end_range: [u8; 40] = concat_arrays!(torrent.key().0, [255; 20]);
                let _: Vec<_> = seeds_map.range(start_range..=end_range).inspect(|_| seeds += 1).collect();
                let _: Vec<_> = peers_map.range(start_range..=end_range).inspect(|_| peers += 1).collect();
                if torrents == size {
                    break;
                }
            }
            start += size;
        }
        (torrents as u64, seeds, peers)
    }
}
