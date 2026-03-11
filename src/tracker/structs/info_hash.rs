/// A 20-byte SHA-1 info hash that uniquely identifies a torrent.
///
/// The inner byte array is stored in binary form. Use the [`Display`] or
/// [`LowerHex`] impl (provided via the `impls` module) to obtain the
/// 40-character hex representation expected by the BitTorrent protocol.
///
/// [`Display`]: std::fmt::Display
/// [`LowerHex`]: std::fmt::LowerHex
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct InfoHash(pub [u8; 20]);