use async_trait::async_trait;
use crate::cache::errors::CacheError;
use crate::tracker::structs::info_hash::InfoHash;

#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn ping(&self) -> Result<(), CacheError>;

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        seeds: u64,
        peers: u64,
        ttl: Option<u64>,
    ) -> Result<(), CacheError>;

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<(u64, u64)>, CacheError>;

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError>;

    async fn set_torrent_peers_batch(
        &self,
        data: &[(InfoHash, u64, u64)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError>;
}