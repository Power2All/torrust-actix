use crate::udp::structs::transaction_id::TransactionId;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ConnectRequest {
    pub transaction_id: TransactionId,
}
