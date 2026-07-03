use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::rtctorrent_bridge::structs::rtc_torrent_bridge::RtcTorrentBridge;
use crate::tracker::structs::announce_entry::AnnounceEntry;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    info,
    warn
};

impl TorrentTracker {
    /// Creates an [`RtcTorrentBridge`] pointing at this tracker's first configured HTTP announce URL.
    pub fn init_rtctorrent_bridge(&self) -> RtcTorrentBridge {
        let tracker_url = if let Some(http_server_config) = self.config.http_server.first() {
            let bind_address = &http_server_config.bind_address;
            if bind_address.contains(':') {
                format!("http://{bind_address}/announce")
            } else {
                format!("http://{bind_address}:6969/announce")
            }
        } else {
            "http://127.0.0.1:6969/announce".to_string()
        };
        RtcTorrentBridge::new(tracker_url)
    }

    /// Builds an RtcTorrent (WebRTC-announced torrent) definition for a local file via the bridge.
    ///
    /// # Errors
    ///
    /// Returns the bridge error when the file cannot be read or the torrent cannot be created.
    pub fn create_rtctorrent(&self, file_path: &str, torrent_name: Option<&str>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let bridge = self.init_rtctorrent_bridge();
        match bridge.create_torrent(file_path, torrent_name) {
            Ok(result) => {
                info!("Successfully created RtcTorrent for file: {file_path}");
                Ok(result)
            },
            Err(e) => {
                warn!("Failed to create RtcTorrent: {e}");
                Err(Box::new(e))
            }
        }
    }

    /// Returns the announce snapshot an RtcTorrent requester should see.
    ///
    /// The requester itself is filtered out; leechers additionally never see other leechers
    /// (only seeders, whose SDP offers they need for signalling).
    pub fn get_rtctorrent_peers(&self, info_hash: InfoHash, requester_is_seed: bool, requester_peer_id: PeerId) -> AnnounceEntry {
        let snapshot = {
            let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
            let lock = shard.read_recursive();
            lock.get(&info_hash).map(AnnounceEntry::from_entry)
        };
        if let Some(mut filtered_entry) = snapshot {
            if requester_is_seed {
                filtered_entry.rtc_seeds.retain(|&peer_id, _| peer_id != requester_peer_id);
                filtered_entry.rtc_peers.retain(|&peer_id, _| peer_id != requester_peer_id);
            } else {
                filtered_entry.rtc_seeds.retain(|&peer_id, _| peer_id != requester_peer_id);
                filtered_entry.rtc_peers.clear();
            }
            filtered_entry
        } else {
            AnnounceEntry::default()
        }
    }

    /// Queues a leecher's SDP answer on the target seeder's pending-answers list.
    ///
    /// The answer is compressed in memory and delivered on the seeder's next announce poll.
    /// Returns `false` when the seeder is not present or has no RTC state.
    pub fn store_rtc_answer(&self, info_hash: InfoHash, seeder_peer_id: PeerId, answerer_peer_id: PeerId, sdp_answer: &str) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&seeder_peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&seeder_peer_id));
            if let Some(seeder) = peer
                && let Some(ref mut rtc) = seeder.rtc_data {
                rtc.pending_answers.push((answerer_peer_id, CompressedBytes::compress(sdp_answer)));
                return true;
            }
        }
        false
    }

    /// Drains and returns all pending SDP answers queued for the given peer, decompressed.
    ///
    /// Subsequent calls return an empty vector until new answers arrive.
    pub fn take_rtc_pending_answers(&self, info_hash: InfoHash, peer_id: PeerId) -> Vec<(PeerId, String)> {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(p) = peer
                && let Some(ref mut rtc) = p.rtc_data {
                return std::mem::take(&mut rtc.pending_answers)
                    .into_iter()
                    .map(|(id, cb)| (id, cb.decompress()))
                    .collect();
            }
        }
        Vec::new()
    }

    /// Stores the accepted SDP answer on the peer and marks its connection as `connected`.
    ///
    /// Returns `false` when the peer is not present or has no RTC state.
    pub fn update_rtc_sdp_answer(&self, info_hash: InfoHash, peer_id: PeerId, sdp_answer: String) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(torrent_peer) = peer
                && let Some(ref mut rtc) = torrent_peer.rtc_data {
                rtc.sdp_answer = Some(CompressedBytes::compress(&sdp_answer));
                rtc.connection_status = "connected".to_string();
                return true;
            }
        }
        false
    }

    /// Replaces the peer's SDP offer (compressed in memory) and marks its connection as `offered`.
    ///
    /// Returns `false` when the peer is not present or has no RTC state.
    pub fn update_rtc_sdp_offer(&self, info_hash: InfoHash, peer_id: PeerId, sdp_offer: &str) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(torrent_peer) = peer
                && let Some(ref mut rtc) = torrent_peer.rtc_data {
                rtc.sdp_offer = Some(CompressedBytes::compress(sdp_offer));
                rtc.connection_status = "offered".to_string();
                return true;
            }
        }
        false
    }

    /// Sets the free-form connection status string on the peer's RTC state.
    ///
    /// Returns `false` when the peer is not present or has no RTC state.
    pub fn update_rtc_connection_status(&self, info_hash: InfoHash, peer_id: PeerId, status: String) -> bool {
        let shard = self.torrents_sharding.get_shard(info_hash.0[0]).unwrap();
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(torrent_peer) = peer
                && let Some(ref mut rtc) = torrent_peer.rtc_data {
                rtc.connection_status = status;
                return true;
            }
        }
        false
    }
}