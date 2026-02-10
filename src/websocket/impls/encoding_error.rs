use crate::websocket::enums::encoding_error::EncodingError;

impl std::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            EncodingError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for EncodingError {}