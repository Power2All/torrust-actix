use crate::tracker::enums::snapshot_maps::SnapshotMaps;
use crate::tracker::structs::announce_entry::{
    AnnounceEntry,
    SNAPSHOT_PEER_CAP,
    SNAPSHOT_RTC_PEER_CAP
};
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::response_peer::ResponsePeer;
use crate::tracker::structs::torrent_counts::TorrentCounts;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::types::peer_map::PeerMap;
use std::time::Instant;

impl AnnounceEntry {
    /// Builds an announce snapshot from a live [`TorrentEntry`], copying only the maps `include`
    /// asks for.
    ///
    /// This runs inside the shard write lock on every announce, so it copies as little as it can:
    /// each map is capped (`SNAPSHOT_PEER_CAP` / `SNAPSHOT_RTC_PEER_CAP`), the BitTorrent maps keep only the fields a
    /// response reads, and the half the caller will not look at is skipped entirely. `counts`
    /// always carries the exact full-swarm totals.
    pub fn from_entry(entry: &TorrentEntry, include: SnapshotMaps) -> Self {
        let bt = matches!(include, SnapshotMaps::Bt | SnapshotMaps::Both);
        let rtc = matches!(include, SnapshotMaps::Rtc | SnapshotMaps::Both);
        AnnounceEntry {
            seeds: if bt { bounded_response_peers(&entry.seeds) } else { Vec::new() },
            seeds_ipv6: if bt { bounded_response_peers(&entry.seeds_ipv6) } else { Vec::new() },
            peers: if bt { bounded_response_peers(&entry.peers) } else { Vec::new() },
            peers_ipv6: if bt { bounded_response_peers(&entry.peers_ipv6) } else { Vec::new() },
            rtc_seeds: if rtc { bounded_clone(&entry.rtc_seeds) } else { PeerMap::default() },
            rtc_peers: if rtc { bounded_clone(&entry.rtc_peers) } else { PeerMap::default() },
            completed: entry.completed,
            updated: entry.updated,
            counts: TorrentCounts::from_entry(entry),
        }
    }
}

impl Default for AnnounceEntry {
    fn default() -> Self {
        AnnounceEntry {
            seeds: Vec::new(),
            seeds_ipv6: Vec::new(),
            peers: Vec::new(),
            peers_ipv6: Vec::new(),
            rtc_seeds: PeerMap::default(),
            rtc_peers: PeerMap::default(),
            completed: 0,
            updated: Instant::now(),
            counts: TorrentCounts::default(),
        }
    }
}

/// Copies at most `SNAPSHOT_PEER_CAP` peers out of a BitTorrent peer map, keeping only the id and
/// address a response reads. One exact-size allocation and no hashing, unlike cloning the map.
#[inline]
fn bounded_response_peers(map: &PeerMap) -> Vec<ResponsePeer> {
    wrapped(map, SNAPSHOT_PEER_CAP)
        .map(|(peer_id, peer)| ResponsePeer { peer_id: *peer_id, peer_addr: peer.peer_addr })
        .collect()
}

/// Copies at most `SNAPSHOT_RTC_PEER_CAP` whole peers, for the RTC maps whose consumers need the
/// signalling state. `CompressedBytes` is `Arc`-backed, so the SDP payloads are refcount bumps.
#[inline]
fn bounded_clone(map: &PeerMap) -> PeerMap {
    if map.len() <= SNAPSHOT_RTC_PEER_CAP {
        return map.clone();
    }
    wrapped(map, SNAPSHOT_RTC_PEER_CAP)
        .map(|(peer_id, peer)| (*peer_id, peer.clone()))
        .collect()
}

/// Up to `cap` entries of `map`, starting at a uniformly random one and wrapping around past the
/// end. Always starting at the front handed every client of a big swarm the same `cap` peers (a
/// map's order only changes as it changes), overloading those and leaving the rest unadvertised.
/// `PeerMap`'s iterator is a slice iterator, so the skip is O(1), not a walk.
#[inline]
fn wrapped(map: &PeerMap, cap: usize) -> impl Iterator<Item = (&PeerId, &TorrentPeer)> {
    let start = if map.len() > cap { rand::RngExt::random_range(&mut rand::rng(), 0..map.len()) } else { 0 };
    map.iter().skip(start).chain(map.iter()).take(map.len().min(cap))
}
