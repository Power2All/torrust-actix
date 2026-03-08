use crate::rtctorrent_bridge::enums::rtc_torrent_bridge_error::RtcTorrentBridgeError;
use crate::rtctorrent_bridge::structs::rtc_torrent_bridge::RtcTorrentBridge;
use serde_json::Value;
use std::process::Command;

impl RtcTorrentBridge {
    pub fn new(tracker_url: String) -> Self {
        Self { tracker_url }
    }

    pub fn create_torrent(&self, file_path: &str, torrent_name: Option<&str>) -> Result<Value, RtcTorrentBridgeError> {
        if !std::path::Path::new(file_path).exists() {
            return Err(RtcTorrentBridgeError::FileNotFoundError(
                format!("File does not exist: {file_path}")
            ));
        }
        let js_command = format!(
            r"
            const fs = require('fs');
            const path = require('path');
            const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
            const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;
            (async () => {{
                try {{
                    const client = new RtcTorrent({{
                        trackerUrl: '{}'
                    }});
                    const stat = fs.statSync('{}');
                    const fileObj = {{
                        path: '{}',
                        name: path.basename('{}'),
                        size: stat.size
                    }};
                    const result = await client.create([fileObj], {{
                        name: '{}'
                    }});
                    console.log(JSON.stringify(result));
                }} catch (error) {{
                    console.error('Error creating torrent:', error);
                    process.exit(1);
                }}
            }})();
            ",
            self.tracker_url,
            file_path,
            file_path,
            file_path,
            torrent_name.unwrap_or("Generated Torrent")
        );
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
        let result_str = String::from_utf8(output.stdout)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))?;
        serde_json::from_str(&result_str)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))
    }

    pub fn seed_torrent(&self, torrent_path: &str) -> Result<Value, RtcTorrentBridgeError> {
        if !std::path::Path::new(torrent_path).exists() {
            return Err(RtcTorrentBridgeError::FileNotFoundError(
                format!("Torrent file does not exist: {torrent_path}")
            ));
        }
        let js_command = format!(
            r"
            const fs = require('fs');
            const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
            const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;
            (async () => {{
                try {{
                    const client = new RtcTorrent({{
                        trackerUrl: '{}',
                        announceInterval: 30000,
                        rtcInterval: 10000
                    }});
                    const torrentBuffer = fs.readFileSync('{}');
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
            ",
            self.tracker_url,
            torrent_path
        );
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
        let result_str = String::from_utf8(output.stdout)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))?;
        serde_json::from_str(&result_str)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))
    }

    pub fn download_torrent(&self, torrent_identifier: &str) -> Result<Value, RtcTorrentBridgeError> {
        let js_command = if torrent_identifier.starts_with("magnet:") {
            format!(
                r"
                const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
                const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;
                (async () => {{
                    try {{
                        const client = new RtcTorrent({{
                            trackerUrl: '{}',
                            announceInterval: 30000,
                            rtcInterval: 10000
                        }});
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
                ",
                self.tracker_url,
                torrent_identifier
            )
        } else {
            if !std::path::Path::new(torrent_identifier).exists() {
                return Err(RtcTorrentBridgeError::FileNotFoundError(
                    format!("Torrent file does not exist: {torrent_identifier}")
                ));
            }
            format!(
                r"
                const fs = require('fs');
                const RtcTorrentLib = require('./lib/rtctorrent/dist/rtctorrent.node.js');
                const RtcTorrent = RtcTorrentLib.default || RtcTorrentLib.RtcTorrent || RtcTorrentLib;
                (async () => {{
                    try {{
                        const client = new RtcTorrent({{
                            trackerUrl: '{}',
                            announceInterval: 30000,
                            rtcInterval: 10000
                        }});
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
                ",
                self.tracker_url,
                torrent_identifier
            )
        };
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
        let result_str = String::from_utf8(output.stdout)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))?;
        serde_json::from_str(&result_str)
            .map_err(|e| RtcTorrentBridgeError::JsonParseError(e.to_string()))
    }
}