use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::AHashMap;
use crate::tracker::structs::torrent_peer::TorrentPeer;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct TorrentPeers {
    pub seeds_ipv4: AHashMap<PeerId, TorrentPeer>,
    pub seeds_ipv6: AHashMap<PeerId, TorrentPeer>,
    pub peers_ipv4: AHashMap<PeerId, TorrentPeer>,
    pub peers_ipv6: AHashMap<PeerId, TorrentPeer>,
}