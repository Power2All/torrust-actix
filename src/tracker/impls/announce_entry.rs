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
use crate::tracker::types::ahash_map::AHashMap;
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
            rtc_seeds: if rtc { bounded_clone(&entry.rtc_seeds) } else { AHashMap::default() },
            rtc_peers: if rtc { bounded_clone(&entry.rtc_peers) } else { AHashMap::default() },
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

/// Copies at most `SNAPSHOT_PEER_CAP` peers out of a BitTorrent peer map, keeping only the id and
/// address a response reads. One exact-size allocation and no hashing, unlike cloning the map.
#[inline]
fn bounded_response_peers(map: &AHashMap<PeerId, TorrentPeer>) -> Vec<ResponsePeer> {
    let mut out = Vec::with_capacity(map.len().min(SNAPSHOT_PEER_CAP));
    for (peer_id, peer) in map.iter().take(SNAPSHOT_PEER_CAP) {
        out.push(ResponsePeer { peer_id: *peer_id, peer_addr: peer.peer_addr });
    }
    out
}

/// Copies at most `SNAPSHOT_RTC_PEER_CAP` whole peers, for the RTC maps whose consumers need the
/// signalling state. `CompressedBytes` is `Arc`-backed, so the SDP payloads are refcount bumps.
#[inline]
fn bounded_clone(map: &AHashMap<PeerId, TorrentPeer>) -> AHashMap<PeerId, TorrentPeer> {
    if map.len() <= SNAPSHOT_RTC_PEER_CAP {
        return map.clone();
    }
    let mut out: AHashMap<PeerId, TorrentPeer> = AHashMap::default();
    out.reserve(SNAPSHOT_RTC_PEER_CAP);
    for (peer_id, peer) in map.iter().take(SNAPSHOT_RTC_PEER_CAP) {
        out.insert(*peer_id, peer.clone());
    }
    out
}
