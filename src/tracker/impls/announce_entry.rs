use crate::tracker::structs::announce_entry::{
    AnnounceEntry,
    SNAPSHOT_PEER_CAP
};
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_counts::TorrentCounts;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::types::ahash_map::AHashMap;
use std::time::Instant;

impl AnnounceEntry {
    /// Builds an announce snapshot from a live [`TorrentEntry`].
    ///
    /// Every peer map, RTC included, is capped at `SNAPSHOT_PEER_CAP` entries (enough to build
    /// a 72-peer response) so the per-announce cost does not grow with swarm size. `counts`
    /// carries the exact full-swarm totals, captured under the same lock.
    pub fn from_entry(entry: &TorrentEntry) -> Self {
        AnnounceEntry {
            seeds: bounded_clone(&entry.seeds),
            seeds_ipv6: bounded_clone(&entry.seeds_ipv6),
            peers: bounded_clone(&entry.peers),
            peers_ipv6: bounded_clone(&entry.peers_ipv6),
            rtc_seeds: bounded_clone(&entry.rtc_seeds),
            rtc_peers: bounded_clone(&entry.rtc_peers),
            completed: entry.completed,
            updated: entry.updated,
            counts: TorrentCounts::from_entry(entry),
        }
    }
}

impl Default for AnnounceEntry {
    fn default() -> Self {
        AnnounceEntry {
            seeds: AHashMap::default(),
            seeds_ipv6: AHashMap::default(),
            peers: AHashMap::default(),
            peers_ipv6: AHashMap::default(),
            rtc_seeds: AHashMap::default(),
            rtc_peers: AHashMap::default(),
            completed: 0,
            updated: Instant::now(),
            counts: TorrentCounts {
                seeds_ipv4: 0,
                seeds_ipv6: 0,
                peers_ipv4: 0,
                peers_ipv6: 0,
                rtc_seeds: 0,
                rtc_peers: 0,
                completed: 0,
            },
        }
    }
}

#[inline]
fn bounded_clone(map: &AHashMap<PeerId, TorrentPeer>) -> AHashMap<PeerId, TorrentPeer> {
    if map.len() <= SNAPSHOT_PEER_CAP {
        return map.clone();
    }
    let mut out: AHashMap<PeerId, TorrentPeer> = AHashMap::default();
    out.reserve(SNAPSHOT_PEER_CAP);
    for (peer_id, peer) in map.iter().take(SNAPSHOT_PEER_CAP) {
        out.insert(*peer_id, peer.clone());
    }
    out
}