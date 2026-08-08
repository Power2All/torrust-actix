#[cfg(test)]
mod tracker_tests {
    use crate::common::structs::number_of_bytes::NumberOfBytes;
    use crate::config::structs::configuration::Configuration;
    use crate::tracker::enums::announce_event::AnnounceEvent;
    use crate::tracker::structs::info_hash::InfoHash;
    use crate::tracker::structs::peer_id::PeerId;
    use crate::tracker::structs::torrent_peer::TorrentPeer;
    use crate::tracker::structs::announce_query_request::AnnounceQueryRequest;
    use crate::tracker::structs::torrent_tracker::TorrentTracker;
    use crate::tracker::structs::user_entry_item::UserEntryItem;
    use crate::tracker::structs::user_id::UserId;
    use std::collections::BTreeMap;
    use std::net::{
        IpAddr,
        SocketAddr
    };
    use std::str::FromStr;
    use std::sync::Arc;

    /// Builds an in-memory tracker. `max_torrents` of 0 keeps the unlimited default.
    async fn tracker(max_torrents: u64) -> TorrentTracker {
        tracker_with(max_torrents, false).await
    }

    /// As [`tracker`], but `persistent` turns on the update queue so a test can assert that a
    /// refused announce never reaches it.
    async fn tracker_with(max_torrents: u64, persistent: bool) -> TorrentTracker {
        let mut config = Configuration::init();
        config.database.path = "sqlite::memory:".to_string();
        config.database.persistent = persistent;
        config.tracker_config.max_torrents = max_torrents;
        config.tracker_config.users_enabled = true;
        TorrentTracker::new(Arc::new(config), false).await
    }

    /// Adds a user and returns its lookup id, for asserting that refused announces leave the
    /// user's activity and counters alone.
    fn add_test_user(tracker: &TorrentTracker) -> UserId {
        let user_id = UserId([7u8; 20]);
        tracker.add_user(user_id, UserEntryItem {
            key: user_id,
            user_id: Some(1),
            user_uuid: None,
            uploaded: 0,
            downloaded: 0,
            completed: 0,
            updated: 0,
            active: 1,
            torrents_active: BTreeMap::new(),
        });
        user_id
    }

    fn announce(info_hash: InfoHash, peer_id: PeerId, event: AnnounceEvent, left: u64) -> AnnounceQueryRequest {
        AnnounceQueryRequest {
            info_hash,
            peer_id,
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left,
            compact: false,
            no_peer_id: false,
            event,
            remote_addr: IpAddr::from_str("10.0.0.1").unwrap(),
            numwant: 72,
            rtctorrent: None,
            rtcoffer: None,
            rtcrequest: None,
            rtcanswer: None,
            rtcanswerfor: None,
        }
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn max_torrents_holds_under_concurrent_distinct_info_hashes() {
        // A cap of 1 puts every task on the boundary at once, which is what makes the race
        // observable: distinct info-hashes land on distinct shards, so nothing serialises the
        // admission check behind a shared lock. Read-then-increment lets most of these through.
        const CAP: u64 = 1;
        const TASKS: u8 = 128;
        // Repeated because a lost race is probabilistic, not guaranteed on any single pass.
        for round in 0..20u8 {
            let tracker = Arc::new(tracker(CAP).await);
            let barrier = Arc::new(tokio::sync::Barrier::new(TASKS as usize));
            let mut tasks = Vec::with_capacity(TASKS as usize);
            for slot in 0..TASKS {
                let tracker = Arc::clone(&tracker);
                let barrier = Arc::clone(&barrier);
                tasks.push(tokio::spawn(async move {
                    let mut raw = [0u8; 20];
                    raw[0] = slot;
                    barrier.wait().await;
                    tracker.add_torrent_peer(InfoHash(raw), PeerId(raw), peer(0), false);
                }));
            }
            for task in tasks {
                task.await.unwrap();
            }

            let tracked = (0..TASKS)
                .filter(|&slot| {
                    let mut raw = [0u8; 20];
                    raw[0] = slot;
                    tracker.get_torrent(InfoHash(raw)).is_some()
                })
                .count() as u64;
            assert_eq!(tracked, CAP, "round {round}: admitted {tracked} torrents against a cap of {CAP}");
            assert_eq!(tracker.get_stats().torrents, CAP as i64, "round {round}: counter drifted from the map");
        }
    }

    #[tokio::test]
    async fn refused_started_announce_touches_no_persistence_or_user_state() {
        let tracker = tracker_with(1, true).await;
        let user_id = add_test_user(&tracker);

        // Fill the single slot, then measure from there.
        tracker.handle_announce(&announce(hash(1), PeerId([1u8; 20]), AnnounceEvent::Started, 100), Some(user_id)).await.unwrap();
        let updates_before = tracker.get_stats().torrents_updates;

        let refused = tracker.handle_announce(&announce(hash(2), PeerId([2u8; 20]), AnnounceEvent::Started, 100), Some(user_id)).await;
        assert!(refused.is_err(), "an announce past max_torrents must be reported to the client");

        assert!(tracker.get_torrent(hash(2)).is_none());
        // The whole point of the cap: a refused announce must not turn into a queued database
        // write, or a flood just moves from memory into the persistence queue.
        assert_eq!(tracker.get_stats().torrents_updates, updates_before, "refused announce queued a persistence update");
        let user = tracker.get_user(user_id).unwrap();
        assert!(!user.torrents_active.contains_key(&hash(2)), "refused announce recorded user activity");
        assert_eq!(user.torrents_active.len(), 1, "only the admitted torrent belongs in torrents_active");
    }

    #[tokio::test]
    async fn refused_completed_announce_credits_nothing() {
        let tracker = tracker_with(1, true).await;
        let user_id = add_test_user(&tracker);

        tracker.handle_announce(&announce(hash(1), PeerId([1u8; 20]), AnnounceEvent::Started, 100), Some(user_id)).await.unwrap();
        let updates_before = tracker.get_stats().torrents_updates;
        let completed_before = tracker.get_stats().completed;

        let refused = tracker.handle_announce(&announce(hash(2), PeerId([2u8; 20]), AnnounceEvent::Completed, 0), Some(user_id)).await;
        assert!(refused.is_err());

        assert!(tracker.get_torrent(hash(2)).is_none());
        assert_eq!(tracker.get_stats().torrents_updates, updates_before);
        assert_eq!(tracker.get_stats().completed, completed_before, "refused announce counted a download");
        assert_eq!(tracker.get_user(user_id).unwrap().completed, 0, "refused announce credited the user");
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
