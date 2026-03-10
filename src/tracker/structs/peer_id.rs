/// A 20-byte peer identifier as defined by the BitTorrent protocol.
///
/// Clients typically encode their name and version in the first few bytes
/// (e.g. `-TR3000-` for Transmission 3.0).  The remaining bytes are random.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct PeerId(pub [u8; 20]);