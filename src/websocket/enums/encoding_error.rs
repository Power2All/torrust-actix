#[derive(Debug)]
pub enum EncodingError {
    SerializationError(String),
    DeserializationError(String),
}