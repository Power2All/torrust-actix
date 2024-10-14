use crate::tracker::structs::info_hash::InfoHash;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::transaction_id::TransactionId;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ScrapeRequest {
    pub connection_id: ConnectionId,
    pub transaction_id: TransactionId,
    pub info_hashes: Vec<InfoHash>,
}