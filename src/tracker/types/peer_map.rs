use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use ahash::AHasher;
use indexmap::IndexMap;
use std::hash::BuildHasherDefault;

/// One of a torrent's peer maps. An `IndexMap` rather than a `HashMap` for its O(1) access by
/// position, which lets an announce start its peer list at a uniformly random peer without
/// walking the map; remove with `swap_remove`, which is O(1) too.
pub type PeerMap = IndexMap<PeerId, TorrentPeer, BuildHasherDefault<AHasher>>;
