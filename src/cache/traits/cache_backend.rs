use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;

#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn ping(&self) -> Result<(), CacheError>;

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        counts: &TorrentPeerCounts,
        ttl: Option<u64>,
    ) -> Result<(), CacheError>;

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<TorrentPeerCounts>, CacheError>;

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError>;

    async fn set_torrent_peers_batch(
        &self,
        data: &[(InfoHash, TorrentPeerCounts)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError>;
}