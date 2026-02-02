use crate::cache::errors::CacheError;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;
use log::debug;
use redis::AsyncCommands;

impl CacheConnectorRedis {
    pub async fn connect(url: &str, prefix: &str) -> Result<Self, CacheError> {
        let client = redis::Client::open(url)
            .map_err(|e| CacheError::ConnectionError(format!("Failed to create Redis client: {}", e)))?;
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::ConnectionError(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self {
            connection,
            prefix: prefix.to_string(),
        })
    }
    
    fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }
}

#[async_trait]
impl CacheBackend for CacheConnectorRedis {
    async fn ping(&self) -> Result<(), CacheError> {
        let mut conn = self.connection.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;
        Ok(())
    }

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        seeds: u64,
        peers: u64,
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        let mut conn = self.connection.clone();
        let key = self.torrent_key(info_hash);
        conn.hset_multiple::<_, _, _, ()>(&key, &[("s", seeds), ("p", peers)])
            .await
            .map_err(CacheError::RedisError)?;
        if let Some(ttl_secs) = ttl
            && ttl_secs > 0 {
                conn.expire::<_, ()>(&key, ttl_secs as i64)
                    .await
                    .map_err(CacheError::RedisError)?;
            }
        debug!("[Redis] Set torrent {} seeds={} peers={}", info_hash, seeds, peers);
        Ok(())
    }

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<(u64, u64)>, CacheError> {
        let mut conn = self.connection.clone();
        let key = self.torrent_key(info_hash);
        let (seeds, peers): (Option<u64>, Option<u64>) = redis::cmd("HMGET")
            .arg(&key)
            .arg("s")
            .arg("p")
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        match (seeds, peers) {
            (Some(s), Some(p)) => Ok(Some((s, p))),
            _ => Ok(None),
        }
    }

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError> {
        let mut conn = self.connection.clone();
        let key = self.torrent_key(info_hash);
        conn.del::<_, ()>(&key)
            .await
            .map_err(CacheError::RedisError)?;
        debug!("[Redis] Deleted torrent {}", info_hash);
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
        let mut conn = self.connection.clone();
        let mut pipe = redis::pipe();
        for (info_hash, seeds, peers) in data {
            let key = self.torrent_key(info_hash);
            pipe.hset_multiple(&key, &[("s", *seeds), ("p", *peers)]);
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
}