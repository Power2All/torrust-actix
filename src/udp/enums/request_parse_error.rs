use std::borrow::Cow;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::transaction_id::TransactionId;

#[derive(Debug)]
pub enum RequestParseError {
    Sendable {
        connection_id: ConnectionId,
        transaction_id: TransactionId,
        err: Cow<'static, str>, // Use Cow for efficient string handling
    },
    Unsendable {
        err: Cow<'static, str>,
    },
}