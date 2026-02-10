use crate::websocket::enums::forward_error::ForwardError;

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForwardError::NotConnected => write!(f, "Not connected to master"),
            ForwardError::Timeout => write!(f, "Cluster timeout"),
            ForwardError::MasterError(msg) => write!(f, "Master error: {}", msg),
            ForwardError::ConnectionLost => write!(f, "Cluster connection lost"),
            ForwardError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for ForwardError {}