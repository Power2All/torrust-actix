use crate::tracker::structs::padded_atomic_u64::PaddedAtomicU64;

pub struct CleanupStatsAtomic {
    pub torrents: PaddedAtomicU64,
    pub seeds: PaddedAtomicU64,
    pub peers: PaddedAtomicU64,
}