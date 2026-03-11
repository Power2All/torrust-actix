use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use crate::tracker::types::ahash_map::AHashMap;
use serde::Serialize;
use std::time::Instant;

/// In-memory state for a single torrent.
///
/// Peers are split into six maps to allow O(1) lookup without filtering:
///
/// | Map | Contents |
/// |-----|----------|
/// | `seeds` / `seeds_ipv6` | IPv4/IPv6 seeders (left == 0) |
/// | `peers` / `peers_ipv6` | IPv4/IPv6 leechers |
/// | `rtc_seeds` / `rtc_peers` | WebRTC seeders and leechers |
///
/// The peer maps are excluded from serialisation (`skip_serializing`) to keep
/// API responses compact — only `completed` and `updated` are serialised.
#[derive(Serialize, Clone, Debug)]
pub struct TorrentEntry {
    /// IPv4 seeders (peers with `left == 0`).
    #[serde(skip_serializing)]
    pub seeds: AHashMap<PeerId, TorrentPeer>,
    /// IPv6 seeders.
    #[serde(skip_serializing)]
    pub seeds_ipv6: AHashMap<PeerId, TorrentPeer>,
    /// IPv4 leechers (peers still downloading).
    #[serde(skip_serializing)]
    pub peers: AHashMap<PeerId, TorrentPeer>,
    /// IPv6 leechers.
    #[serde(skip_serializing)]
    pub peers_ipv6: AHashMap<PeerId, TorrentPeer>,
    /// WebRTC seeders.
    #[serde(skip_serializing)]
    pub rtc_seeds: AHashMap<PeerId, TorrentPeer>,
    /// WebRTC leechers.
    #[serde(skip_serializing)]
    pub rtc_peers: AHashMap<PeerId, TorrentPeer>,
    /// Total number of times this torrent has been fully downloaded.
    pub completed: u64,
    /// Monotonic timestamp of the last announce that modified this entry.
    #[serde(with = "serde_millis")]
    pub updated: Instant
}