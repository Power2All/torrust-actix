#[cfg(test)]
mod webtorrent_tests {
    use crate::webtorrent::structs::webtorrent_peer::WebTorrentPeer;

    #[test]
    fn test_webtorrent_peer_creation() {
        let peer_id = [1u8; 20];
        let addr = "127.0.0.1:6881".parse().unwrap();
        let peer = WebTorrentPeer::new(peer_id, addr);
        assert_eq!(peer.peer_id, peer_id);
        assert_eq!(peer.peer_addr, addr);
        assert_eq!(peer.uploaded, 0);
        assert_eq!(peer.downloaded, 0);
        assert!(peer.offer.is_none());
    }

    #[test]
    fn test_webtorrent_peer_update() {
        let peer_id = [1u8; 20];
        let addr = "127.0.0.1:6881".parse().unwrap();
        let mut peer = WebTorrentPeer::new(peer_id, addr);
        peer.update(1000, 500, 0);
        assert_eq!(peer.uploaded, 1000);
        assert_eq!(peer.downloaded, 500);
        assert_eq!(peer.left, 0);
        assert_eq!(peer.is_seeder, Some(true));
    }

    #[test]
    fn test_webtorrent_peer_offer() {
        let peer_id = [1u8; 20];
        let addr = "127.0.0.1:6881".parse().unwrap();
        let mut peer = WebTorrentPeer::new(peer_id, addr);
        peer.set_offer("sdp_offer_data".to_string(), "offer_123".to_string());
        assert_eq!(peer.offer, Some("sdp_offer_data".to_string()));
        assert_eq!(peer.offer_id, Some("offer_123".to_string()));
    }

    #[test]
    fn test_generate_offer_id() {
        let addr = "127.0.0.1:6881".parse().unwrap();
        let mut full_peer_id = [0u8; 20];
        full_peer_id[0] = 0xAB;
        full_peer_id[1] = 0xCD;
        full_peer_id[2] = 0xEF;
        full_peer_id[3..].copy_from_slice(&[0xEF; 17]);
        let peer = WebTorrentPeer::new(full_peer_id, addr);
        let offer_id = peer.generate_offer_id();
        assert!(offer_id.len() > 10);
    }
}