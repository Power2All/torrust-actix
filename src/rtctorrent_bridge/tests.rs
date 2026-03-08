use crate::rtctorrent_bridge::structs::rtc_torrent_bridge::RtcTorrentBridge;

#[test]
fn test_bridge_creation() {
    let tracker_url = "http://localhost:6969/announce".to_string();
    let bridge = RtcTorrentBridge::new(tracker_url);
    assert_eq!(bridge.tracker_url, "http://localhost:6969/announce");
}
