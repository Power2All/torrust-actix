use crate::tracker::types::ahash_map::AHashMap;
use smallvec::SmallVec;

pub type QueryValues = SmallVec<[Vec<u8>; 1]>;

/// A parsed URL query string.
///
/// Uses the project's [`AHashMap`], which is safe here despite the keys coming straight off the
/// wire: ahash calls `BuildHasherDefault<AHasher>` "fixed keys", but with the `std`/`runtime-rng`
/// features (both on by default) those keys are drawn from `getrandom` on first use and held in a
/// process-wide `OnceBox`. "Fixed" means identical across every map *within one run*, not
/// hardcoded — two runs of this binary hash the same info-hash to different values, so colliding
/// keys cannot be worked out ahead of time.
pub type QueryMap = AHashMap<String, QueryValues>;
