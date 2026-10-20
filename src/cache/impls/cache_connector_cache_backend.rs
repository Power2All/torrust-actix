use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector::CacheConnector;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::tracker::structs::info_hash::InfoHash;

/// Forwards each call to the connected engine.
///
/// Every method is the same two-arm match, because [`CacheConnector`] can only ever be one engine
/// or the other — there is no "configured but not connected" state left to handle.
macro_rules! dispatch {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            CacheConnector::Redis(backend) => backend.$method($($arg),*).await,
            CacheConnector::Memcache(backend) => backend.$method($($arg),*).await,
        }
    };
}

impl CacheConnector {

    pub async fn delete_torrents(&self, info_hashes: &[InfoHash]) -> Result<(), CacheError> {
        dispatch!(self, delete_torrents, info_hashes)
    }

    pub async fn set_torrent_peers_batch(
        &self,
        data: &[(InfoHash, TorrentPeerCounts)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        dispatch!(self, set_torrent_peers_batch, data, ttl)
    }
}
