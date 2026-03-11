/// A 20-byte identifier for a registered tracker user.
///
/// Stored in the same compact binary format as [`InfoHash`] and [`PeerId`].
/// Typically derived from a SHA-1 hash of the user's access key string.
///
/// [`InfoHash`]: crate::tracker::structs::info_hash::InfoHash
/// [`PeerId`]: crate::tracker::structs::peer_id::PeerId
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct UserId(pub [u8; 20]);