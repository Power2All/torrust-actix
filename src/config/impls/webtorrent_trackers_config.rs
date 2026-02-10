use crate::config::structs::webtorrent_trackers_config::WebTorrentTrackersConfig;

impl Default for WebTorrentTrackersConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: "0.0.0.0:12100".to_string(),
            keep_alive: 60,
            request_timeout: 10,
            disconnect_timeout: 10,
            max_connections: 100,
            threads: 4,
            ssl: false,
            ssl_key: String::new(),
            ssl_cert: String::new(),
            tls_connection_rate: 100,
        }
    }
}