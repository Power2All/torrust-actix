/// A signed 64-bit byte count used for the `uploaded`, `downloaded`, and
/// `left` fields in announce requests.
///
/// The BitTorrent protocol transmits these as unsigned integers, but SQLx maps
/// them to `i64` for database storage — hence the signed inner type.
#[derive(PartialEq, PartialOrd, Eq, Hash, Clone, Copy, Debug)]
pub struct NumberOfBytes(pub i64);