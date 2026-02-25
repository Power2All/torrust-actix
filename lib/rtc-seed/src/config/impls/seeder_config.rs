use crate::config::structs::seeder_config::SeederConfig;

impl Default for SeederConfig {
    fn default() -> Self {
        Self {
            tracker_url: "http://127.0.0.1:6969/announce".to_string(),
            file_paths: Vec::new(),
            name: None,
            out_file: None,
            webseed_urls: Vec::new(),
            ice_servers: vec!["stun:stun.l.google.com:19302".to_string()],
            rtc_interval_ms: 5000,
        }
    }
}