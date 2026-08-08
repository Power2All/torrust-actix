#[cfg(test)]
mod tracker_tests {
    use crate::common::structs::number_of_bytes::NumberOfBytes;
    use crate::config::structs::configuration::Configuration;
    use crate::tracker::enums::announce_event::AnnounceEvent;
    use crate::tracker::structs::info_hash::InfoHash;
    use crate::tracker::structs::peer_id::PeerId;
    use crate::tracker::structs::torrent_peer::TorrentPeer;
    use crate::tracker::structs::torrent_tracker::TorrentTracker;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::sync::Arc;

    /// Builds an in-memory tracker. `max_torrents` of 0 keeps the unlimited default.
    async fn tracker(max_torrents: u64) -> TorrentTracker {
        let mut config = Configuration::init();
        config.database.path = "sqlite::memory:".to_string();
        config.database.persistent = false;
        config.tracker_config.max_torrents = max_torrents;
        TorrentTracker::new(Arc::new(config), false).await
    }

    fn peer(left: u64) -> TorrentPeer {
        TorrentPeer {
            peer_id: PeerId([1u8; 20]),
            peer_addr: SocketAddr::from_str("10.0.0.1:6881").unwrap(),
            updated: std::time::Instant::now(),
            uploaded: NumberOfBytes(0),
            downloaded: NumberOfBytes(0),
            left: NumberOfBytes(left as i64),
            event: AnnounceEvent::Started,
            rtc_data: None,
        }
    }

    fn hash(byte: u8) -> InfoHash {
        InfoHash([byte; 20])
    }

    #[tokio::test]
    async fn completed_counts_the_transition_to_seeding_once() {
        let tracker = tracker(0).await;
        let info_hash = hash(1);
        let peer_id = PeerId([1u8; 20]);

        // Leech first, so the torrent exists and the peer is not yet a seed.
        tracker.add_torrent_peer(info_hash, peer_id, peer(100), false);
        assert_eq!(tracker.get_torrent(info_hash).unwrap().completed, 0);

        // Finishing the download counts once.
        tracker.add_torrent_peer(info_hash, peer_id, peer(0), true);
        assert_eq!(tracker.get_torrent(info_hash).unwrap().completed, 1);

        // Replaying `event=completed` must not keep inflating the figure: the peer is already
        // a seed, so there is no new transition to count.
        for _ in 0..10 {
            tracker.add_torrent_peer(info_hash, peer_id, peer(0), true);
        }
        assert_eq!(tracker.get_torrent(info_hash).unwrap().completed, 1);
    }

    #[tokio::test]
    async fn completed_ignores_a_peer_that_is_still_leeching() {
        let tracker = tracker(0).await;
        let info_hash = hash(2);

        // `completed` asserted while `left != 0` is a lie, and used to be counted anyway.
        tracker.add_torrent_peer(info_hash, PeerId([2u8; 20]), peer(500), true);
        assert_eq!(tracker.get_torrent(info_hash).unwrap().completed, 0);
    }

    #[tokio::test]
    async fn max_torrents_refuses_new_swarms_but_not_known_ones() {
        let tracker = tracker(2).await;

        tracker.add_torrent_peer(hash(1), PeerId([1u8; 20]), peer(0), false);
        tracker.add_torrent_peer(hash(2), PeerId([2u8; 20]), peer(0), false);
        assert!(tracker.get_torrent(hash(1)).is_some());
        assert!(tracker.get_torrent(hash(2)).is_some());

        // At the cap: a third info-hash is not created.
        tracker.add_torrent_peer(hash(3), PeerId([3u8; 20]), peer(0), false);
        assert!(tracker.get_torrent(hash(3)).is_none());

        // Torrents already tracked keep accepting peers, or the cap would freeze the swarm.
        tracker.add_torrent_peer(hash(1), PeerId([9u8; 20]), peer(50), false);
        let entry = tracker.get_torrent(hash(1)).unwrap();
        assert_eq!(entry.seeds.len() + entry.peers.len(), 2);
    }

    #[tokio::test]
    async fn max_torrents_zero_is_unlimited() {
        let tracker = tracker(0).await;
        for i in 0..16u8 {
            tracker.add_torrent_peer(hash(i), PeerId([i; 20]), peer(0), false);
            assert!(tracker.get_torrent(hash(i)).is_some());
        }
    }
}
