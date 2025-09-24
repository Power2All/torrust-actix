use std::sync::atomic::AtomicU64;
use crate::tracker::impls::torrent_sharding::CACHE_LINE_SIZE;

#[repr(align(64))]
pub struct PaddedAtomicU64 {
    pub value: AtomicU64,
    pub _padding: [u8; CACHE_LINE_SIZE - std::mem::size_of::<AtomicU64>()],
}