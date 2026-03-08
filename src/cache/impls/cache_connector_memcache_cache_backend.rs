use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;
use log::debug;

#[async_trait]
impl CacheBackend for CacheConnectorMemcache {
    async fn ping(&self) -> Result<(), CacheError> {
        let client = self.client.lock();
        client.version()
            .map_err(CacheError::MemcacheError)?;
        Ok(())
    }

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        seeds: u64,
        peers: u64,
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        let client = self.client.lock();
        let key = self.torrent_key(info_hash);
        let value = Self::serialize_peers(seeds, peers);
        let expiration = ttl.unwrap_or(0) as u32;
        client.set(&key, value.as_str(), expiration)
            .map_err(CacheError::MemcacheError)?;
        debug!("[Memcache] Set torrent {info_hash} seeds={seeds} peers={peers}");
        Ok(())
    }

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<(u64, u64)>, CacheError> {
        let client = self.client.lock();
        let key = self.torrent_key(info_hash);
        match client.get::<String>(&key) {
            Ok(Some(value)) => Ok(Self::deserialize_peers(&value)),
            Ok(None) => Ok(None),
            Err(e) => Err(CacheError::MemcacheError(e)),
        }
    }

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError> {
        let client = self.client.lock();
        let key = self.torrent_key(info_hash);
        let _ = client.delete(&key);
        debug!("[Memcache] Deleted torrent {info_hash}");
        Ok(())
    }

    async fn set_torrent_peers_batch(
        &self,
        data: &[(InfoHash, u64, u64)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        if data.is_empty() {
            return Ok(());
        }
        let client = self.client.lock();
        let expiration = ttl.unwrap_or(0) as u32;
        for (info_hash, seeds, peers) in data {
            let key = self.torrent_key(info_hash);
            let value = Self::serialize_peers(*seeds, *peers);
            client.set(&key, value.as_str(), expiration)
                .map_err(CacheError::MemcacheError)?;
        }
        debug!("[Memcache] Batch set {} torrents", data.len());
        Ok(())
    }
}