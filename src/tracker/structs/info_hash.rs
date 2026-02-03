//! BitTorrent info hash identifier.

/// A 20-byte BitTorrent info hash.
///
/// The info hash is the SHA-1 hash of the "info" dictionary in a torrent file.
/// It uniquely identifies a torrent across the BitTorrent network.
///
/// # Structure
///
/// The info hash is exactly 20 bytes, matching the SHA-1 digest size.
/// The first byte is used for sharding in the tracker's storage system.
///
/// # Example
///
/// ```rust
/// use torrust_actix::tracker::structs::info_hash::InfoHash;
///
/// // Create from a 20-byte array
/// let hash = InfoHash([0u8; 20]);
///
/// // Access the underlying bytes
/// let bytes: &[u8; 20] = &hash.0;
/// ```
///
/// # Serialization
///
/// The info hash is typically represented as a 40-character hexadecimal string
/// or URL-encoded in tracker requests.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct InfoHash(pub [u8; 20]);