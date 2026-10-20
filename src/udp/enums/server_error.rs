use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("internal server error")]
    InternalServerError,
    #[error("torrent not on whitelist")]
    TorrentNotWhitelisted,
    #[error("torrent blacklist")]
    TorrentBlacklisted,
    #[error("unknown key")]
    UnknownKey,
    #[error("connection id missing or expired")]
    InvalidConnectionId,
    #[error("invalid authentication key")]
    PeerKeyNotValid,
    #[error("bad request")]
    BadRequest,
}