//! Bridge between Rust Torrust tracker and JavaScript RtcTorrent library
//!
//! This module provides a seamless interface between the Rust tracker backend
//! and the JavaScript RtcTorrent library to enable WebRTC-powered BitTorrent.

use std::process::Command;
use serde_json::Value;

/// Represents the RtcTorrent client interface
pub struct RtcTorrentBridge {
    tracker_url: String,
}

#[derive(Debug)]
pub enum RtcTorrentBridgeError {
    CommandExecutionError(String),
    JsonParseError(String),
    FileNotFoundError(String),
    ValidationError(String),
}

impl std::fmt::Display for RtcTorrentBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RtcTorrentBridgeError::CommandExecutionError(msg) => write!(f, "Command execution error: {}", msg),
            RtcTorrentBridgeError::JsonParseError(msg) => write!(f, "JSON parse error: {}", msg),
            RtcTorrentBridgeError::FileNotFoundError(msg) => write!(f, "File not found: {}", msg),
            RtcTorrentBridgeError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for RtcTorrentBridgeError {}

impl RtcTorrentBridge {
    /// Creates a new RtcTorrent bridge instance
    pub fn new(tracker_url: String) -> Self {
        Self { tracker_url }
    }

    /// Creates a torrent from a given file path using the JavaScript RtcTorrent library
    pub fn create_torrent(&self, file_path: &str, torrent_name: Option<&str>) -> Result<Value, RtcTorrentBridgeError> {
        // Verify the file exists
        if !std::path::Path::new(file_path).exists() {
            return Err(RtcTorrentBridgeError::FileNotFoundError(
                format!("File does not exist: {}", file_path)
            ));
        }

        // Build the JavaScript command to create the torrent
        let js_command = format!(
            r#"
            const fs = require('fs');
            const path = require('path');

            // Load the Node.js RtcTorrent library
            const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
            const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;

            (async () => {{
                try {{
                    const client = new RtcTorrent({{
                        trackerUrl: '{}'
                    }});

                    // Get file information
                    const stat = fs.statSync('{}');
                    const fileObj = {{
                        path: '{}',
                        name: path.basename('{}'),
                        size: stat.size
                    }};

                    // Create the torrent
                    const result = await client.create([fileObj], {{
                        name: '{}'
                    }});

                    console.log(JSON.stringify(result));
                }} catch (error) {{
                    console.error('Error creating torrent:', error);
                    process.exit(1);
                }}
            }})();
            "#,
            self.tracker_url,
            file_path,
            file_path,
            file_path,
            torrent_name.unwrap_or("Generated Torrent")
        );

        // Execute the Node.js command
        let output = Command::new("node")
            .arg("-e")
            .arg(js_command)
            .output()
            .map_err(|e| RtcTorrentBridgeError::CommandExecutionError(e.to_string()))?;

        if !output.status.success() {
            return Err(RtcTorrentBridgeError::CommandExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Parse the result
        let result_str = String::from_utf8(output.stdout)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))?;

        serde_json::from_str(&result_str)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))
    }

    /// Seeds a torrent from the given file using the JavaScript RtcTorrent library
    pub fn seed_torrent(&self, torrent_path: &str) -> Result<Value, RtcTorrentBridgeError> {
        // Verify the torrent file exists
        if !std::path::Path::new(torrent_path).exists() {
            return Err(RtcTorrentBridgeError::FileNotFoundError(
                format!("Torrent file does not exist: {}", torrent_path)
            ));
        }

        // Build the JavaScript command to seed the torrent
        let js_command = format!(
            r#"
            const fs = require('fs');

            // Load the Node.js RtcTorrent library
            const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
            const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;

            (async () => {{
                try {{
                    const client = new RtcTorrent({{
                        trackerUrl: '{}',
                        announceInterval: 30000,
                        rtcInterval: 10000
                    }});

                    // Read the torrent file
                    const torrentBuffer = fs.readFileSync('{}');

                    // Seed the torrent
                    const torrent = await client.seed(torrentBuffer);

                    console.log(JSON.stringify({{
                        success: true,
                        infoHash: torrent.data.infoHash,
                        peers: torrent.peers.size
                    }}));
                }} catch (error) {{
                    console.error('Error seeding torrent:', error);
                    process.exit(1);
                }}
            }})();
            "#,
            self.tracker_url,
            torrent_path
        );

        // Execute the Node.js command
        let output = Command::new("node")
            .arg("-e")
            .arg(js_command)
            .output()
            .map_err(|e| RtcTorrentBridgeError::CommandExecutionError(e.to_string()))?;

        if !output.status.success() {
            return Err(RtcTorrentBridgeError::CommandExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Parse the result
        let result_str = String::from_utf8(output.stdout)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))?;

        serde_json::from_str(&result_str)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))
    }

    /// Downloads a torrent using the JavaScript RtcTorrent library
    pub fn download_torrent(&self, torrent_identifier: &str) -> Result<Value, RtcTorrentBridgeError> {
        // Determine if the identifier is a magnet URI or a file path
        let js_command = if torrent_identifier.starts_with("magnet:") {
            // It's a magnet URI
            format!(
                r#"
                // Load the Node.js RtcTorrent library
                const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
                const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;

                (async () => {{
                    try {{
                        const client = new RtcTorrent({{
                            trackerUrl: '{}',
                            announceInterval: 30000,
                            rtcInterval: 10000
                        }});

                        // Download using magnet URI
                        const torrent = await client.download('{}');

                        console.log(JSON.stringify({{
                            success: true,
                            infoHash: torrent.data.infoHash,
                            downloaded: torrent.downloaded,
                            totalSize: torrent.totalSize
                        }}));
                    }} catch (error) {{
                        console.error('Error downloading torrent:', error);
                        process.exit(1);
                    }}
                }})();
                "#,
                self.tracker_url,
                torrent_identifier
            )
        } else {
            // It's a file path - verify it exists
            if !std::path::Path::new(torrent_identifier).exists() {
                return Err(RtcTorrentBridgeError::FileNotFoundError(
                    format!("Torrent file does not exist: {}", torrent_identifier)
                ));
            }

            // Build command for torrent file
            format!(
                r#"
                const fs = require('fs');

                // Load the Node.js RtcTorrent library
                const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
                const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;

                (async () => {{
                    try {{
                        const client = new RtcTorrent({{
                            trackerUrl: '{}',
                            announceInterval: 30000,
                            rtcInterval: 10000
                        }});

                        // Read the torrent file and download
                        const torrentBuffer = fs.readFileSync('{}');
                        const torrent = await client.download(torrentBuffer);

                        console.log(JSON.stringify({{
                            success: true,
                            infoHash: torrent.data.infoHash,
                            downloaded: torrent.downloaded,
                            totalSize: torrent.totalSize
                        }}));
                    }} catch (error) {{
                        console.error('Error downloading torrent:', error);
                        process.exit(1);
                    }}
                }})();
                "#,
                self.tracker_url,
                torrent_identifier
            )
        };

        // Execute the Node.js command
        let output = Command::new("node")
            .arg("-e")
            .arg(js_command)
            .output()
            .map_err(|e| RtcTorrentBridgeError::CommandExecutionError(e.to_string()))?;

        if !output.status.success() {
            return Err(RtcTorrentBridgeError::CommandExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Parse the result
        let result_str = String::from_utf8(output.stdout)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))?;

        serde_json::from_str(&result_str)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let tracker_url = "http://localhost:6969/announce".to_string();
        let bridge = RtcTorrentBridge::new(tracker_url);
        assert_eq!(bridge.tracker_url, "http://localhost:6969/announce");
    }
}