use crate::rtctorrent_bridge::enums::rtc_torrent_bridge_error::RtcTorrentBridgeError;
use std::fmt;

impl fmt::Display for RtcTorrentBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RtcTorrentBridgeError::CommandExecutionError(msg) => write!(f, "Command execution error: {msg}"),
            RtcTorrentBridgeError::JsonParseError(msg) => write!(f, "JSON parse error: {msg}"),
            RtcTorrentBridgeError::FileNotFoundError(msg) => write!(f, "File not found: {msg}"),
            RtcTorrentBridgeError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for RtcTorrentBridgeError {}