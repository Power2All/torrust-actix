use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector::CacheConnector;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;

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

#[async_trait]
impl CacheBackend for CacheConnector {
    async fn ping(&self) -> Result<(), CacheError> {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("cache_ping", "cache");
        let result = dispatch!(self, ping);
        if let Some(txn) = transaction {
            match &result {
                Ok(()) => txn.set_tag("result", "success"),
                Err(e) => txn.set_tag("result", format!("error: {e:?}")),
            }
            txn.set_tag("engine", format!("{:?}", self.engine()));
            txn.finish();
        }
        result
    }

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        counts: &TorrentPeerCounts,
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        dispatch!(self, set_torrent_peers, info_hash, counts, ttl)
    }

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<TorrentPeerCounts>, CacheError> {
        dispatch!(self, get_torrent_peers, info_hash)
    }

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError> {
        dispatch!(self, delete_torrent, info_hash)
    }

    async fn delete_torrents(&self, info_hashes: &[InfoHash]) -> Result<(), CacheError> {
        dispatch!(self, delete_torrents, info_hashes)
    }

    async fn set_torrent_peers_batch(
        &self,
        data: &[(InfoHash, TorrentPeerCounts)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        dispatch!(self, set_torrent_peers_batch, data, ttl)
    }
}
