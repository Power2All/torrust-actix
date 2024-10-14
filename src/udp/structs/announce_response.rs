use crate::udp::structs::announce_interval::AnnounceInterval;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::traits::Ip;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AnnounceResponse<I: Ip> {
    pub transaction_id: TransactionId,
    pub announce_interval: AnnounceInterval,
    pub leechers: NumberOfPeers,
    pub seeders: NumberOfPeers,
    pub peers: Vec<ResponsePeer<I>>,
}