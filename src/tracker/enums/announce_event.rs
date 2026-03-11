use serde::Deserialize;

/// The `event` field of a BitTorrent announce request ([BEP 3]).
///
/// Clients send `started` on their first announce for a torrent, `completed`
/// when the download finishes, and `stopped` when they shut down.  All other
/// announces omit the field (represented here as `None`).
///
/// [BEP 3]: https://www.bittorrent.org/beps/bep_0003.html
#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AnnounceEvent {
    /// Regular re-announce — no special event.
    None = 0,
    /// The client has finished downloading the torrent.
    Completed = 1,
    /// The client is starting to download/seed the torrent.
    Started = 2,
    /// The client is shutting down for this torrent.
    Stopped = 3
}