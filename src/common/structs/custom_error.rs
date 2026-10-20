/// A lightweight error type carrying a plain-text message.
///
/// Used throughout the tracker for query-parsing failures, validation errors,
/// and other recoverable conditions that do not need a full error-chain.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct CustomError {
    pub(crate) message: String,
}