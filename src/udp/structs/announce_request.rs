use std::net::Ipv4Addr;
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::peer_key::PeerKey;
use crate::udp::structs::port::Port;
use crate::udp::structs::transaction_id::TransactionId;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AnnounceRequest {
    pub connection_id: ConnectionId,
    pub transaction_id: TransactionId,
    pub info_hash: InfoHash,
    pub peer_id: PeerId,
    pub bytes_downloaded: NumberOfBytes,
    pub bytes_uploaded: NumberOfBytes,
    pub bytes_left: NumberOfBytes,
    pub event: AnnounceEvent,
    pub ip_address: Option<Ipv4Addr>,
    pub key: PeerKey,
    pub peers_wanted: NumberOfPeers,
    pub port: Port,
    pub path: String,
}
