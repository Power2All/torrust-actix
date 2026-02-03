//! Separated IPv4/IPv6 peer collections.

use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::AHashMap;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use serde::Serialize;

/// Peer collections separated by IP version and peer type.
///
/// This struct organizes peers into four categories for efficient announce
/// response generation. BitTorrent clients typically only want peers of
/// the same IP version as themselves.
///
/// # Structure
///
/// Peers are organized into four maps:
/// - `seeds_ipv4`: IPv4 peers with complete copy
/// - `seeds_ipv6`: IPv6 peers with complete copy
/// - `peers_ipv4`: IPv4 peers still downloading
/// - `peers_ipv6`: IPv6 peers still downloading
///
/// # Usage
///
/// This struct is typically used when returning peers from an announce request,
/// allowing the tracker to filter by IP version (BEP 7: IPv6 Tracker Extension).
///
/// # Example
///
/// ```rust,ignore
/// use torrust_actix::tracker::structs::torrent_peers::TorrentPeers;
///
/// // Get IPv4 peers only
/// let ipv4_count = peers.seeds_ipv4.len() + peers.peers_ipv4.len();
///
/// // Get total seed count
/// let total_seeds = peers.seeds_ipv4.len() + peers.seeds_ipv6.len();
/// ```
#[derive(Serialize, Debug)]
pub struct TorrentPeers {
    /// IPv4 seeding peers (complete copy).
    pub seeds_ipv4: AHashMap<PeerId, TorrentPeer>,

    /// IPv6 seeding peers (complete copy).
    pub seeds_ipv6: AHashMap<PeerId, TorrentPeer>,

    /// IPv4 downloading peers (incomplete).
    pub peers_ipv4: AHashMap<PeerId, TorrentPeer>,

    /// IPv6 downloading peers (incomplete).
    pub peers_ipv6: AHashMap<PeerId, TorrentPeer>,
}