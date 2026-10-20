use crate::tracker::structs::torrent_peer::TorrentPeer;
use std::net::IpAddr;

impl TorrentPeer {

    /// Whether an announce from `ip` carrying `key` may update or remove this stored peer: it must
    /// come from the same IP, or prove itself with the same announce key. Peer ids are no secret
    /// (non-compact responses list them, and every BitTorrent handshake reveals one), so without
    /// this anyone could send `event=stopped` for a swarm's peers or repoint them at another address.
    pub fn is_same_client(&self, ip: IpAddr, key: Option<u32>) -> bool {
        self.peer_addr.ip() == ip || (self.key.is_some() && self.key == key)
    }

    /// Returns `true` when this peer announced via RtcTorrent (carries WebRTC signalling state).
    pub fn is_rtctorrent(&self) -> bool {
        self.rtc_data.is_some()
    }

    /// Returns the peer's SDP offer, decompressed on demand.
    pub fn rtc_sdp_offer(&self) -> Option<String> {
        self.rtc_data.as_ref()?.sdp_offer.as_ref().map(|cb| cb.decompress())
    }

}