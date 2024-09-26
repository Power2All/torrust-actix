use std::collections::BTreeMap;
use serde::Serialize;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;

#[derive(Serialize, Clone, Debug)]
pub struct TorrentEntry {
    #[serde(skip_serializing)]
    pub seeds: BTreeMap<PeerId, TorrentPeer>,
    #[serde(skip_serializing)]
    pub peers: BTreeMap<PeerId, TorrentPeer>,
    pub completed: u64,
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant
}