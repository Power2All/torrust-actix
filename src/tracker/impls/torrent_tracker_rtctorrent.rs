use crate::rtctorrent_bridge::RtcTorrentBridge;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::types::ahash_map::AHashMap;
use log::{info, warn};

impl TorrentTracker {
    pub fn init_rtctorrent_bridge(&self) -> RtcTorrentBridge {
        let tracker_url = if let Some(http_server_config) = self.config.http_server.first() {
            let bind_address = &http_server_config.bind_address;
            if bind_address.contains(":") {
                format!("http://{}/announce", bind_address)
            } else {
                format!("http://{}:6969/announce", bind_address)
            }
        } else {
            "http://127.0.0.1:6969/announce".to_string()
        };
        RtcTorrentBridge::new(tracker_url)
    }

    pub fn create_rtctorrent(&self, file_path: &str, torrent_name: Option<&str>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let bridge = self.init_rtctorrent_bridge();
        match bridge.create_torrent(file_path, torrent_name) {
            Ok(result) => {
                info!("Successfully created RtcTorrent for file: {}", file_path);
                Ok(result)
            },
            Err(e) => {
                warn!("Failed to create RtcTorrent: {}", e);
                Err(Box::new(e))
            }
        }
    }

    pub fn get_rtctorrent_peers(&self, info_hash: InfoHash, requester_is_seed: bool, requester_peer_id: PeerId) -> TorrentEntry {
        if let Some(torrent_entry) = self.get_torrent(info_hash) {
            let mut filtered_entry = torrent_entry.clone();
            if requester_is_seed {
                filtered_entry.rtc_seeds.retain(|&peer_id, _| peer_id != requester_peer_id);
                filtered_entry.rtc_peers.retain(|&peer_id, _| peer_id != requester_peer_id);
            } else {
                filtered_entry.rtc_seeds.retain(|&peer_id, _| peer_id != requester_peer_id);
                filtered_entry.rtc_peers.clear();
            }

            filtered_entry
        } else {
            TorrentEntry {
                seeds: AHashMap::default(),
                seeds_ipv6: AHashMap::default(),
                peers: AHashMap::default(),
                peers_ipv6: AHashMap::default(),
                rtc_seeds: AHashMap::default(),
                rtc_peers: AHashMap::default(),
                completed: 0,
                updated: std::time::Instant::now()
            }
        }
    }

    pub fn store_rtc_answer(&self, info_hash: InfoHash, seeder_peer_id: PeerId, answerer_peer_id: PeerId, sdp_answer: String) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&seeder_peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&seeder_peer_id));
            if let Some(seeder) = peer {
                seeder.rtc_pending_answers.push((answerer_peer_id, sdp_answer));
                return true;
            }
        }
        false
    }

    pub fn take_rtc_pending_answers(&self, info_hash: InfoHash, peer_id: PeerId) -> Vec<(PeerId, String)> {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(p) = peer {
                return std::mem::take(&mut p.rtc_pending_answers);
            }
        }
        Vec::new()
    }

    pub fn update_rtc_sdp_answer(&self, info_hash: InfoHash, peer_id: PeerId, sdp_answer: String) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            if let Some(torrent_peer) = torrent_entry.rtc_seeds.get_mut(&peer_id) {
                torrent_peer.rtc_sdp_answer = Some(sdp_answer);
                torrent_peer.rtc_connection_status = "connected".to_string();
                true
            } else if let Some(torrent_peer) = torrent_entry.rtc_peers.get_mut(&peer_id) {
                torrent_peer.rtc_sdp_answer = Some(sdp_answer);
                torrent_peer.rtc_connection_status = "connected".to_string();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn update_rtc_sdp_offer(&self, info_hash: InfoHash, peer_id: PeerId, sdp_offer: String) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            if let Some(torrent_peer) = torrent_entry.rtc_seeds.get_mut(&peer_id) {
                torrent_peer.rtc_sdp_offer = Some(sdp_offer);
                torrent_peer.rtc_connection_status = "offered".to_string();
                true
            } else if let Some(torrent_peer) = torrent_entry.rtc_peers.get_mut(&peer_id) {
                torrent_peer.rtc_sdp_offer = Some(sdp_offer);
                torrent_peer.rtc_connection_status = "offered".to_string();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn update_rtc_connection_status(&self, info_hash: InfoHash, peer_id: PeerId, status: String) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            if let Some(torrent_peer) = torrent_entry.rtc_seeds.get_mut(&peer_id) {
                torrent_peer.rtc_connection_status = status;
                true
            } else if let Some(torrent_peer) = torrent_entry.rtc_peers.get_mut(&peer_id) {
                torrent_peer.rtc_connection_status = status;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}