/// Which peer maps an [`AnnounceEntry`] snapshot needs to carry.
///
/// A snapshot is built under the shard write lock on every announce, so copying maps the caller
/// will not read is the most expensive thing the announce path does. The three callers each want
/// a different half:
///
/// - `add_torrent_peer` serves BitTorrent responses. `handle_announce` rebuilds the snapshot via
///   `get_rtctorrent_peers` for RtcTorrent announces and discards this one, so its RTC maps are
///   never read.
/// - `get_rtctorrent_peers` reads only the RTC maps.
/// - `remove_torrent_peer` backs the `stopped` event, which has no RTC rebuild, so it needs both.
///
/// [`AnnounceEntry`]: crate::tracker::structs::announce_entry::AnnounceEntry
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapshotMaps {
    /// The four BitTorrent peer maps; RTC maps left empty.
    Bt,
    /// The two RtcTorrent peer maps; BitTorrent maps left empty.
    Rtc,
    /// Every peer map.
    Both,
}
