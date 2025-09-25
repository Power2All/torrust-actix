use std::io;
use std::borrow::Cow;
use crate::udp::enums::request_parse_error::RequestParseError;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::transaction_id::TransactionId;

impl RequestParseError {
    #[tracing::instrument(level = "debug")]
    pub fn sendable_io(err: io::Error, connection_id: i64, transaction_id: i32) -> Self {
        Self::Sendable {
            connection_id: ConnectionId(connection_id),
            transaction_id: TransactionId(transaction_id),
            err: Cow::Owned(err.to_string()),
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn sendable_text(text: &'static str, connection_id: i64, transaction_id: i32) -> Self {
        Self::Sendable {
            connection_id: ConnectionId(connection_id),
            transaction_id: TransactionId(transaction_id),
            err: Cow::Borrowed(text),
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn unsendable_io(err: io::Error) -> Self {
        Self::Unsendable {
            err: Cow::Owned(err.to_string()),
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn unsendable_text(text: &'static str) -> Self {
        Self::Unsendable {
            err: Cow::Borrowed(text),
        }
    }
}