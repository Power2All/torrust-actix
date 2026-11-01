use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::cache::structs::user_stat_counts::UserStatCounts;
use crate::tracker::structs::info_hash::InfoHash;
use log::debug;
use redis::AsyncCommands;

impl CacheConnectorRedis {
    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, CacheError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::ConnectionError(format!("Redis connect failed: {e}")))
    }

    pub async fn delete_torrents(&self, info_hashes: &[InfoHash]) -> Result<(), CacheError> {
        if info_hashes.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn().await?;
        let keys: Vec<String> = info_hashes.iter().map(|hash| self.torrent_key(hash)).collect();
        conn.del::<_, ()>(keys)
            .await
            .map_err(CacheError::RedisError)?;
        debug!("[Redis] Deleted {} torrents", info_hashes.len());
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
        let mut conn = self.conn().await?;
        let mut pipe = redis::pipe();
        for (info_hash, counts) in data {
            let key = self.torrent_key(info_hash);
            if self.split_peers {
                pipe.hset_multiple(&key, &[
                    ("bt_seeds_ipv4", counts.bt_seeds_ipv4),
                    ("bt_seeds_ipv6", counts.bt_seeds_ipv6),
                    ("rtc_seeds",     counts.rtc_seeds),
                    ("bt_peers_ipv4", counts.bt_peers_ipv4),
                    ("bt_peers_ipv6", counts.bt_peers_ipv6),
                    ("rtc_peers",     counts.rtc_peers),
                    ("c",             counts.completed),
                ]);
            } else {
                pipe.hset_multiple(&key, &[
                    ("s", counts.total_seeds()),
                    ("p", counts.total_peers()),
                    ("c", counts.completed),
                ]);
            }
            if let Some(ttl_secs) = ttl
                && ttl_secs > 0 {
                    pipe.expire(&key, ttl_secs as i64);
                }
        }
        pipe.query_async::<()>(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;
        debug!("[Redis] Batch set {} torrents", data.len());
        Ok(())
    }

    pub async fn delete_users(&self, ids: &[String]) -> Result<(), CacheError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn().await?;
        let keys: Vec<String> = ids.iter().map(|id| self.user_key(id)).collect();
        conn.del::<_, ()>(keys)
            .await
            .map_err(CacheError::RedisError)?;
        debug!("[Redis] Deleted {} users", ids.len());
        Ok(())
    }

    /// Writes each user's totals as a hash. User keys get no TTL: a user's totals stay valid
    /// while they are not announcing, and the key is deleted when the user is.
    pub async fn set_users_batch(&self, data: &[(String, UserStatCounts)]) -> Result<(), CacheError> {
        if data.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn().await?;
        let mut pipe = redis::pipe();
        for (id, counts) in data {
            pipe.hset_multiple(self.user_key(id), &[
                ("uploaded",   counts.uploaded),
                ("downloaded", counts.downloaded),
                ("completed",  counts.completed),
                ("active",     u64::from(counts.active)),
                ("updated",    counts.updated),
                ("torrents",   counts.torrents_active),
            ]);
        }
        pipe.query_async::<()>(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;
        debug!("[Redis] Batch set {} users", data.len());
        Ok(())
    }
}
