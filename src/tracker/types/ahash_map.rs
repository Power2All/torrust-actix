use ahash::AHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

pub type AHashMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;