#[derive(Debug)]
pub enum ForwardError {
    NotConnected,
    Timeout,
    MasterError(String),
    ConnectionLost,
    EncodingError(String),
}