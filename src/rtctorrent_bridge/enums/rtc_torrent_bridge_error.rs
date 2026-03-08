#[derive(Debug)]
pub enum RtcTorrentBridgeError {
    CommandExecutionError(String),
    JsonParseError(String),
    FileNotFoundError(String),
    ValidationError(String),
}