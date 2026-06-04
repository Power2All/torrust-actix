use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_counts::TorrentCounts;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::types::ahash_map::AHashMap;
use std::time::Instant;

pub const SNAPSHOT_PEER_CAP: usize = 128;

#[derive(Clone, Debug)]
pub struct AnnounceEntry {
    pub seeds: AHashMap<PeerId, TorrentPeer>,
    pub seeds_ipv6: AHashMap<PeerId, TorrentPeer>,
    pub peers: AHashMap<PeerId, TorrentPeer>,
    pub peers_ipv6: AHashMap<PeerId, TorrentPeer>,
    pub rtc_seeds: AHashMap<PeerId, TorrentPeer>,
    pub rtc_peers: AHashMap<PeerId, TorrentPeer>,
    pub completed: u64,
    pub updated: Instant,
    pub counts: TorrentCounts,
}