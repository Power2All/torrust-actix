use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use ahash::AHasher;
use serde::Serialize;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

pub type AHashMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

#[derive(Serialize, Clone, Debug)]
pub struct TorrentEntry {
    #[serde(skip_serializing)]
    pub seeds: AHashMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub peers: AHashMap<PeerId, TorrentPeer>,
    pub completed: u64,
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant
}