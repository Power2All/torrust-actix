use crate::tracker::types::peer_map::PeerMap;
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
    pub seeds: PeerMap,
    /// IPv6 seeders.
    #[serde(skip_serializing)]
    pub seeds_ipv6: PeerMap,
    /// IPv4 leechers (peers still downloading).
    #[serde(skip_serializing)]
    pub peers: PeerMap,
    /// IPv6 leechers.
    #[serde(skip_serializing)]
    pub peers_ipv6: PeerMap,
    /// WebRTC seeders.
    #[serde(skip_serializing)]
    pub rtc_seeds: PeerMap,
    /// WebRTC leechers.
    #[serde(skip_serializing)]
    pub rtc_peers: PeerMap,
    /// Total number of times this torrent has been fully downloaded.
    pub completed: u64,
    /// Monotonic timestamp of the last announce that modified this entry.
    #[serde(with = "serde_millis")]
    pub updated: Instant
}