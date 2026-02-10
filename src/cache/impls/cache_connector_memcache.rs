use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;
use log::debug;
use parking_lot::Mutex;
use std::fmt;
use std::sync::Arc;

impl fmt::Debug for CacheConnectorMemcache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheConnectorMemcache")
            .field("client", &"<memcache::Client>")
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl CacheConnectorMemcache {
    pub fn connect(url: &str, prefix: &str) -> Result<Self, CacheError> {
        let client = memcache::connect(url)
            .map_err(|e| CacheError::ConnectionError(format!("Failed to connect to Memcache: {}", e)))?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            prefix: prefix.to_string(),
        })
    }

    fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }

    fn serialize_peers(seeds: u64, peers: u64) -> String {
        format!("{}:{}", seeds, peers)
    }

    fn deserialize_peers(value: &str) -> Option<(u64, u64)> {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() == 2 {
            let seeds = parts[0].parse::<u64>().ok()?;
            let peers = parts[1].parse::<u64>().ok()?;
            Some((seeds, peers))
        } else {
            None
        }
    }
}

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
        debug!("[Memcache] Set torrent {} seeds={} peers={}", info_hash, seeds, peers);
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
        debug!("[Memcache] Deleted torrent {}", info_hash);
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