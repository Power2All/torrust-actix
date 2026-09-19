use crate::tracker::structs::peer_id::PeerId;
use std::net::SocketAddr;

/// The only two things an announce response needs about another peer.
///
/// [`AnnounceEntry`] used to carry whole [`TorrentPeer`] values, but every BitTorrent response
/// builder reads exactly these fields — the address to pack into the compact peer string, and
/// the id, to skip the requester itself and to fill the `peer id` key of a dictionary response.
/// The counters, the `Instant` and the boxed RTC state were copied and thrown away.
///
/// At 56 bytes against `TorrentPeer`'s 104 this halves the copy, and because these live in a
/// `Vec` rather than a hash map, building a snapshot no longer hashes every peer id.
///
/// RtcTorrent responses still need the full peer, so [`AnnounceEntry`]'s RTC maps keep it.
///
/// [`AnnounceEntry`]: crate::tracker::structs::announce_entry::AnnounceEntry
/// [`TorrentPeer`]: crate::tracker::structs::torrent_peer::TorrentPeer
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResponsePeer {
    /// The peer's client-chosen id.
    pub peer_id: PeerId,
    /// The address and port to hand to other peers.
    pub peer_addr: SocketAddr,
}
