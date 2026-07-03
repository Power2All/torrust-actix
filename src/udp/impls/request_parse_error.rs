use crate::udp::enums::request_parse_error::RequestParseError;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::transaction_id::TransactionId;
use std::borrow::Cow;
use std::io;

impl RequestParseError {
    /// Wraps an I/O parse failure that can still be answered (connection and transaction ids known).
    pub fn sendable_io(err: io::Error, connection_id: i64, transaction_id: i32) -> Self {
        Self::Sendable {
            connection_id: ConnectionId(connection_id),
            transaction_id: TransactionId(transaction_id),
            err: Cow::Owned(err.to_string()),
        }
    }

    /// Wraps a textual parse failure that can still be answered (connection and transaction ids known).
    pub fn sendable_text(text: &'static str, connection_id: i64, transaction_id: i32) -> Self {
        Self::Sendable {
            connection_id: ConnectionId(connection_id),
            transaction_id: TransactionId(transaction_id),
            err: Cow::Borrowed(text),
        }
    }

    /// Wraps an I/O parse failure for which no error response can be sent.
    pub fn unsendable_io(err: io::Error) -> Self {
        Self::Unsendable {
            err: Cow::Owned(err.to_string()),
        }
    }

    /// Wraps a textual parse failure for which no error response can be sent.
    pub fn unsendable_text(text: &'static str) -> Self {
        Self::Unsendable {
            err: Cow::Borrowed(text),
        }
    }
}