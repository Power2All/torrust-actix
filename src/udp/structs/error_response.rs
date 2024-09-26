use std::borrow::Cow;
use crate::udp::structs::transaction_id::TransactionId;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ErrorResponse {
    pub transaction_id: TransactionId,
    pub message: Cow<'static, str>,
}