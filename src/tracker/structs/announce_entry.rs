use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::response_peer::ResponsePeer;
use crate::tracker::structs::torrent_counts::TorrentCounts;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::types::ahash_map::AHashMap;
use std::time::Instant;

/// Upper bound on peers copied out of any one *BitTorrent* map when building a snapshot.
///
/// `numwant` is clamped to `1..=72` on both the HTTP and UDP paths, and a snapshot is taken
/// straight after the requester was inserted into one of these maps — so 72 peers plus the one
/// the response has to skip is everything a full response can use. Anything above that was
/// copied under the shard write lock and then thrown away.
pub const SNAPSHOT_PEER_CAP: usize = 73;

/// Upper bound on peers copied out of an *RtcTorrent* map.
///
/// Deliberately not [`SNAPSHOT_PEER_CAP`]: the signalling response in `http.rs` never looks at
/// `numwant`, it emits every RTC seeder holding a non-empty SDP offer, so the reasoning that
/// bounds the BitTorrent maps at 73 does not apply here. This is the value both caps shared
/// before they were split, kept so the number of signalling partners an RtcTorrent peer can see
/// is unchanged. The shipped client asks for 50.
pub const SNAPSHOT_RTC_PEER_CAP: usize = 128;

/// A bounded, response-shaped view of a torrent, captured under the shard lock.
///
/// The BitTorrent maps hold [`ResponsePeer`] in a `Vec`: they are already segregated by seed/leech
/// and address family, every consumer iterates them linearly, and nothing outside the RTC path
/// reads more than the address and the id. The RtcTorrent maps keep whole [`TorrentPeer`] values
/// because the signalling response needs each peer's SDP state.
///
/// `counts` carries the exact full-swarm totals; the maps are capped, so their lengths are not
/// the swarm size and must never be reported as it.
#[derive(Clone, Debug)]
pub struct AnnounceEntry {
    /// IPv4 seeders.
    pub seeds: Vec<ResponsePeer>,
    /// IPv6 seeders.
    pub seeds_ipv6: Vec<ResponsePeer>,
    /// IPv4 leechers.
    pub peers: Vec<ResponsePeer>,
    /// IPv6 leechers.
    pub peers_ipv6: Vec<ResponsePeer>,
    /// WebRTC seeders, with their signalling state.
    pub rtc_seeds: AHashMap<PeerId, TorrentPeer>,
    /// WebRTC leechers, with their signalling state.
    pub rtc_peers: AHashMap<PeerId, TorrentPeer>,
    /// Times this torrent has been completed.
    pub completed: u64,
    /// When the underlying entry was last modified.
    pub updated: Instant,
    /// Exact swarm totals, captured under the same lock.
    pub counts: TorrentCounts,
}
