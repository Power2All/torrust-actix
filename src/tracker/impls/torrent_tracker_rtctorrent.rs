use crate::common::structs::compressed_bytes::CompressedBytes;
use crate::tracker::enums::snapshot_maps::SnapshotMaps;
use crate::tracker::structs::announce_entry::AnnounceEntry;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    /// Returns the announce snapshot an RtcTorrent requester should see.
    ///
    /// The requester itself is filtered out; leechers additionally never see other leechers
    /// (only seeders, whose SDP offers they need for signalling).
    pub fn get_rtctorrent_peers(&self, info_hash: InfoHash, requester_is_seed: bool, requester_peer_id: PeerId) -> AnnounceEntry {
        let snapshot = {
            let shard = self.torrents_sharding.shard_for(info_hash);
            let lock = shard.read_recursive();
            lock.get(&info_hash).map(|entry| AnnounceEntry::from_entry(entry, SnapshotMaps::Rtc))
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
    ///
    /// The queue is filled by *other* peers naming this one as the target and survives the
    /// target's re-announces, so it is capped at `max_rtc_pending_answers`, dropping the oldest
    /// entry to make room.
    pub fn store_rtc_answer(&self, info_hash: InfoHash, seeder_peer_id: PeerId, answerer_peer_id: PeerId, sdp_answer: &str) -> bool {
        let max_pending = self.config.tracker_config.max_rtc_pending_answers as usize;
        let shard = self.torrents_sharding.shard_for(info_hash);
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&seeder_peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&seeder_peer_id));
            if let Some(seeder) = peer
                && let Some(ref mut rtc) = seeder.rtc_data {
                if let Some(slot) = rtc.pending_answers.iter_mut().find(|(id, _)| *id == answerer_peer_id) {
                    slot.1 = CompressedBytes::compress(sdp_answer);
                    return true;
                }
                while rtc.pending_answers.len() >= max_pending {
                    rtc.pending_answers.remove(0);
                }
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
        let shard = self.torrents_sharding.shard_for(info_hash);
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

    /// Stores the accepted SDP answer on the peer.
    ///
    /// Returns `false` when the peer is not present or has no RTC state.
    pub fn update_rtc_sdp_answer(&self, info_hash: InfoHash, peer_id: PeerId, sdp_answer: String) -> bool {
        let shard = self.torrents_sharding.shard_for(info_hash);
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(torrent_peer) = peer
                && let Some(ref mut rtc) = torrent_peer.rtc_data {
                rtc.sdp_answer = Some(CompressedBytes::compress(&sdp_answer));
                return true;
            }
        }
        false
    }

    /// Replaces the peer's SDP offer, compressed in memory.
    ///
    /// Returns `false` when the peer is not present or has no RTC state.
    pub fn update_rtc_sdp_offer(&self, info_hash: InfoHash, peer_id: PeerId, sdp_offer: &str) -> bool {
        let shard = self.torrents_sharding.shard_for(info_hash);
        let mut lock = shard.write();
        if let Some(torrent_entry) = lock.get_mut(&info_hash) {
            let peer = torrent_entry.rtc_seeds.get_mut(&peer_id)
                .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id));
            if let Some(torrent_peer) = peer
                && let Some(ref mut rtc) = torrent_peer.rtc_data {
                rtc.sdp_offer = Some(CompressedBytes::compress(sdp_offer));
                return true;
            }
        }
        false
    }
}