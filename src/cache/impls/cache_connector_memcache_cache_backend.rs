use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::cache::structs::user_stat_counts::UserStatCounts;
use crate::tracker::structs::info_hash::InfoHash;
use log::debug;

impl CacheConnectorMemcache {
    /// Deletes torrents' keys; memcache has no multi-key delete, so one at a time.
    pub async fn delete_torrents(&self, info_hashes: &[InfoHash]) -> Result<(), CacheError> {
        let client = self.client.lock();
        for info_hash in info_hashes {
            let _ = client.delete(&self.torrent_key(info_hash));
        }
        debug!("[Memcache] Deleted {} torrents", info_hashes.len());
        Ok(())
    }

    pub async fn set_torrent_peers_batch(
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

    /// Deletes users' keys; memcache has no multi-key delete, so one at a time.
    pub async fn delete_users(&self, ids: &[String]) -> Result<(), CacheError> {
        let client = self.client.lock();
        for id in ids {
            let _ = client.delete(&self.user_key(id));
        }
        debug!("[Memcache] Deleted {} users", ids.len());
        Ok(())
    }

    /// Writes each user's totals. User keys never expire, see the Redis backend.
    pub async fn set_users_batch(&self, data: &[(String, UserStatCounts)]) -> Result<(), CacheError> {
        if data.is_empty() {
            return Ok(());
        }
        let client = self.client.lock();
        for (id, counts) in data {
            client.set(&self.user_key(id), Self::serialize_user(counts).as_str(), 0)
                .map_err(CacheError::MemcacheError)?;
        }
        debug!("[Memcache] Batch set {} users", data.len());
        Ok(())
    }
}
