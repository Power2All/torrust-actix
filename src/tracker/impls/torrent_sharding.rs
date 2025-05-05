use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use log::info;
use parking_lot::RwLock;
use tokio::runtime::Builder;
use tokio_shutdown::Shutdown;
use crate::common::common::shutdown_waiting;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

#[allow(dead_code)]
impl TorrentSharding {
    fn default() -> Self {
        Self::new()
    }

    #[tracing::instrument(level = "debug")]
    pub fn new() -> TorrentSharding
    {
        let mut bags: Vec<Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>> = Vec::new();
        for i in 0..=255 {
            bags[i as usize] = Arc::new(RwLock::new(Default::default()));
        }
        TorrentSharding {
            shard_bag: bags,
        }
    }

    pub async fn cleanup_threads(&self, torrent_tracker: Arc<TorrentTracker>, shutdown: Shutdown, peer_timeout: Duration, persistent: bool)
    {
        let tokio_threading = match torrent_tracker.clone().config.tracker_config.peers_cleanup_threads {
            0 => {
                Builder::new_current_thread().thread_name("sharding").enable_all().build().unwrap()
            }
            _ => {
                Builder::new_multi_thread().thread_name("sharding").worker_threads(torrent_tracker.clone().config.tracker_config.peers_cleanup_threads as usize).enable_all().build().unwrap()
            }
        };
        for shard in 0u8..=255u8 {
            let torrent_tracker_clone = torrent_tracker.clone();
            let shutdown_clone = shutdown.clone();
            tokio_threading.spawn(async move {
                loop {
                    if shutdown_waiting(Duration::from_secs(torrent_tracker_clone.clone().config.tracker_config.peers_cleanup_interval), shutdown_clone.clone()).await {
                        return;
                    }

                    let (mut torrents, mut seeds, mut peers) = (0u64, 0u64, 0u64);
                    let shard_data = torrent_tracker_clone.clone().torrents_sharding.get_shard_content(shard);
                    for (info_hash, torrent_entry) in shard_data.iter() {
                        for (peer_id, torrent_peer) in torrent_entry.seeds.iter() {
                            if torrent_peer.updated.elapsed() > peer_timeout {
                                let shard = torrent_tracker_clone.clone().torrents_sharding.get_shard(shard);
                                let mut lock = shard.write();
                                match lock.entry(*info_hash) {
                                    Entry::Vacant(_) => {}
                                    Entry::Occupied(mut o) => {
                                        if o.get_mut().seeds.remove(peer_id).is_some() {
                                            torrent_tracker_clone.clone().update_stats(StatsEvent::Seeds, -1);
                                            seeds += 1;
                                        };
                                        if o.get_mut().peers.remove(peer_id).is_some() {
                                            torrent_tracker_clone.clone().update_stats(StatsEvent::Peers, -1);
                                            peers += 1;
                                        };
                                        if !persistent && o.get().seeds.is_empty() && o.get().peers.is_empty() {
                                            lock.remove(info_hash);
                                            torrent_tracker_clone.clone().update_stats(StatsEvent::Torrents, -1);
                                            torrents += 1;
                                        }
                                    }
                                }
                            }
                        }
                        for (peer_id, torrent_peer) in torrent_entry.peers.iter() {
                            if torrent_peer.updated.elapsed() > peer_timeout {
                                let shard = torrent_tracker_clone.clone().torrents_sharding.get_shard(shard);
                                let mut lock = shard.write();
                                match lock.entry(*info_hash) {
                                    Entry::Vacant(_) => {}
                                    Entry::Occupied(mut o) => {
                                        if o.get_mut().seeds.remove(peer_id).is_some() {
                                            torrent_tracker_clone.clone().update_stats(StatsEvent::Seeds, -1);
                                            seeds += 1;
                                        };
                                        if o.get_mut().peers.remove(peer_id).is_some() {
                                            torrent_tracker_clone.clone().update_stats(StatsEvent::Peers, -1);
                                            peers += 1;
                                        };
                                        if !persistent && o.get().seeds.is_empty() && o.get().peers.is_empty() {
                                            lock.remove(info_hash);
                                            torrent_tracker_clone.clone().update_stats(StatsEvent::Torrents, -1);
                                            torrents += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    info!("[PEERS] Shard: {} - Torrents: {} - Seeds: {} - Peers: {}", shard, torrents, seeds, peers);
                }
            });
        }
        shutdown.clone().handle().await;
        mem::forget(tokio_threading);
    }

    #[tracing::instrument(level = "debug")]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool
    {
        self.get_shard_content(info_hash.0[0]).contains_key(&info_hash)
    }

    #[tracing::instrument(level = "debug")]
    pub fn contains_peer(&self, info_hash: InfoHash, peer_id: PeerId) -> bool
    {
        match self.get_shard_content(info_hash.0[0]).get(&info_hash) {
            None => { false }
            Some(torrent_entry) => {
                if torrent_entry.seeds.contains_key(&peer_id) || torrent_entry.peers.contains_key(&peer_id) {
                    return true;
                }
                false
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    #[allow(unreachable_patterns)]
    pub fn get_shard(&self, shard: u8) -> Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>
    {
        self.shard_bag[shard as usize].clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_shard_content(&self, shard: u8) -> BTreeMap<InfoHash, TorrentEntry>
    {
        self.get_shard(shard).read_recursive().clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_all_content(&self) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let mut torrents_return = BTreeMap::new();
        for index in 0u8..=255u8 {
            let mut shard = self.get_shard(index).read_recursive().clone();
            torrents_return.append(&mut shard);
        }
        torrents_return
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_torrents_amount(&self) -> u64
    {
        let mut torrents = 0u64;
        for index in 0u8..=255u8 {
            torrents += self.get_shard(index).read_recursive().len() as u64;
        }
        torrents
    }
}