use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentEntry {
    #[serde(skip)]
    pub peers: BTreeMap<PeerId, TorrentPeer>,
    #[serde(skip)]
    pub seeds: BTreeMap<PeerId, TorrentPeer>,
    pub completed: i64,
    #[serde(with = "serde_millis")]
    pub updated: std::time::Instant
}
