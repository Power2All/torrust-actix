use crate::tracker::types::peer_map::PeerMap;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct TorrentPeers {
    pub seeds_ipv4: PeerMap,
    pub seeds_ipv6: PeerMap,
    pub peers_ipv4: PeerMap,
    pub peers_ipv6: PeerMap,
}