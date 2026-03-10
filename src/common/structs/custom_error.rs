/// A lightweight error type carrying a plain-text message.
///
/// Used throughout the tracker for query-parsing failures, validation errors,
/// and other recoverable conditions that do not need a full error-chain.
/// Implements [`std::error::Error`] and [`std::fmt::Display`] via the `impls`
/// module.
#[derive(Debug)]
pub struct CustomError {
    pub(crate) message: String,
}