use std::collections::BTreeMap;
use serde::Serialize;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_peer::TorrentPeer;

#[derive(Serialize, Debug)]
pub struct TorrentPeers {
    pub seeds_ipv4: BTreeMap<PeerId, TorrentPeer>,
    pub seeds_ipv6: BTreeMap<PeerId, TorrentPeer>,
    pub peers_ipv4: BTreeMap<PeerId, TorrentPeer>,
    pub peers_ipv6: BTreeMap<PeerId, TorrentPeer>,
}
