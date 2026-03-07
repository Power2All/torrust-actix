use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use serde::Serialize;
use std::time::Instant;
use crate::tracker::types::ahash_map::AHashMap;

#[derive(Serialize, Clone, Debug)]
pub struct TorrentEntry {
    #[serde(skip_serializing)]
    pub seeds: AHashMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub seeds_ipv6: AHashMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub peers: AHashMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub peers_ipv6: AHashMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub rtc_seeds: AHashMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub rtc_peers: AHashMap<PeerId, TorrentPeer>,
    pub completed: u64,
    #[serde(with = "serde_millis")]
    pub updated: Instant
}

impl TorrentEntry {
    pub fn new() -> Self {
        TorrentEntry {
            seeds: AHashMap::default(),
            seeds_ipv6: AHashMap::default(),
            peers: AHashMap::default(),
            peers_ipv6: AHashMap::default(),
            rtc_seeds: AHashMap::default(),
            rtc_peers: AHashMap::default(),
            completed: 0,
            updated: Instant::now(),
        }
    }
}