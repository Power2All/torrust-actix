use crate::webtorrent::structs::webtorrent_peer::WebTorrentPeer;
use std::net::SocketAddr;
use std::time::{
    Instant,
    SystemTime,
    UNIX_EPOCH
};

impl WebTorrentPeer {
    pub fn new(peer_id: [u8; 20], peer_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            peer_addr,
            uploaded: 0,
            downloaded: 0,
            left: u64::MAX,
            offer: None,
            offer_id: None,
            last_announce: now,
            first_announce: now,
            is_seeder: None,
        }
    }

    pub fn update(&mut self, uploaded: u64, downloaded: u64, left: u64) {
        self.uploaded = uploaded;
        self.downloaded = downloaded;
        self.left = left;
        self.last_announce = Instant::now();
        self.is_seeder = Some(left == 0);
    }

    pub fn set_offer(&mut self, offer: String, offer_id: String) {
        self.offer = Some(offer);
        self.offer_id = Some(offer_id);
    }

    pub fn clear_offer(&mut self) {
        self.offer = None;
        self.offer_id = None;
    }

    pub fn is_timeout(&self, timeout_seconds: u64) -> bool {
        self.last_announce.elapsed().as_secs() > timeout_seconds
    }

    pub fn seconds_since_last_announce(&self) -> u64 {
        self.last_announce.elapsed().as_secs()
    }

    pub fn generate_offer_id(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("{}-{:x}", timestamp, self.peer_id[0])
    }
}