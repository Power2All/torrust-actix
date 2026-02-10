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
    pub peers: AHashMap<PeerId, TorrentPeer>,
    pub completed: u64,
    #[serde(with = "serde_millis")]
    pub updated: Instant
}