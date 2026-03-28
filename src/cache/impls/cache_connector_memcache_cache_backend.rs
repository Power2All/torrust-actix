use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
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
        counts: &TorrentPeerCounts,
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        let client = self.client.lock();
        let key = self.torrent_key(info_hash);
        let value = if self.split_peers {
            Self::serialize_split(counts)
        } else {
            Self::serialize_aggregated(counts.total_seeds(), counts.total_peers(), counts.completed)
        };
        let expiration = ttl.unwrap_or(0) as u32;
        client.set(&key, value.as_str(), expiration)
            .map_err(CacheError::MemcacheError)?;
        debug!("[Memcache] Set torrent {info_hash} counts={counts:?}");
        Ok(())
    }

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<TorrentPeerCounts>, CacheError> {
        let client = self.client.lock();
        let key = self.torrent_key(info_hash);
        match client.get::<String>(&key) {
            Ok(Some(value)) => {
                if self.split_peers {
                    Ok(Self::deserialize_split(&value))
                } else {
                    Ok(Self::deserialize_aggregated(&value).map(|(s, p, c)| TorrentPeerCounts {
                        bt_seeds_ipv4: s,
                        bt_peers_ipv4: p,
                        completed: c,
                        ..Default::default()
                    }))
                }
            }
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
        data: &[(InfoHash, TorrentPeerCounts)],
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        if data.is_empty() {
            return Ok(());
        }
        let client = self.client.lock();
        let expiration = ttl.unwrap_or(0) as u32;
        for (info_hash, counts) in data {
            let key = self.torrent_key(info_hash);
            let value = if self.split_peers {
                Self::serialize_split(counts)
            } else {
                Self::serialize_aggregated(counts.total_seeds(), counts.total_peers(), counts.completed)
            };
            client.set(&key, value.as_str(), expiration)
                .map_err(CacheError::MemcacheError)?;
        }
        debug!("[Memcache] Batch set {} torrents", data.len());
        Ok(())
    }
}
