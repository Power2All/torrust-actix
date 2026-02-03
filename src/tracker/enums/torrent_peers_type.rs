//! Peer filtering by IP version.

/// Filter type for selecting peers by IP version.
///
/// Used when returning peers from announce requests to filter by IP address
/// family. This implements support for BEP 7 (IPv6 Tracker Extension).
///
/// # Variants
///
/// - **All**: Return both IPv4 and IPv6 peers
/// - **IPv4**: Return only IPv4 peers
/// - **IPv6**: Return only IPv6 peers
///
/// # Protocol Support
///
/// - IPv4 clients typically receive IPv4 peers by default
/// - IPv6 clients may receive IPv6 peers if available (BEP 7)
/// - Some clients request both using multiple announce requests
///
/// # Example
///
/// ```rust
/// use torrust_actix::tracker::enums::torrent_peers_type::TorrentPeersType;
///
/// let filter = TorrentPeersType::IPv4;
/// // Use filter to select which peers to return
/// ```
#[derive(Debug)]
pub enum TorrentPeersType {
    /// Return all peers regardless of IP version.
    All,

    /// Return only IPv4 peers.
    IPv4,

    /// Return only IPv6 peers.
    IPv6,
}