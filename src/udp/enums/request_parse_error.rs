use std::io;
use actix_web::Either;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::transaction_id::TransactionId;

#[derive(Debug)]
pub enum RequestParseError {
    Sendable {
        connection_id: ConnectionId,
        transaction_id: TransactionId,
        err: Either<io::Error, &'static str>,
    },
    Unsendable {
        err: Either<io::Error, &'static str>,
    },
}