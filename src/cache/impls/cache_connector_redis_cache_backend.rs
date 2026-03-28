use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::cache::structs::torrent_peer_counts::TorrentPeerCounts;
use crate::cache::traits::cache_backend::CacheBackend;
use crate::tracker::structs::info_hash::InfoHash;
use async_trait::async_trait;
use log::debug;
use redis::AsyncCommands;

impl CacheConnectorRedis {
    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, CacheError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::ConnectionError(format!("Redis connect failed: {e}")))
    }
}

#[async_trait]
impl CacheBackend for CacheConnectorRedis {
    async fn ping(&self) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;
        Ok(())
    }

    async fn set_torrent_peers(
        &self,
        info_hash: &InfoHash,
        counts: &TorrentPeerCounts,
        ttl: Option<u64>,
    ) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        let key = self.torrent_key(info_hash);
        if self.split_peers {
            conn.hset_multiple::<_, _, _, ()>(&key, &[
                ("bt_seeds_ipv4", counts.bt_seeds_ipv4),
                ("bt_seeds_ipv6", counts.bt_seeds_ipv6),
                ("rtc_seeds",     counts.rtc_seeds),
                ("bt_peers_ipv4", counts.bt_peers_ipv4),
                ("bt_peers_ipv6", counts.bt_peers_ipv6),
                ("rtc_peers",     counts.rtc_peers),
                ("c",             counts.completed),
            ]).await.map_err(CacheError::RedisError)?;
        } else {
            conn.hset_multiple::<_, _, _, ()>(&key, &[
                ("s", counts.total_seeds()),
                ("p", counts.total_peers()),
                ("c", counts.completed),
            ]).await.map_err(CacheError::RedisError)?;
        }
        if let Some(ttl_secs) = ttl
            && ttl_secs > 0 {
                conn.expire::<_, ()>(&key, ttl_secs as i64)
                    .await
                    .map_err(CacheError::RedisError)?;
            }
        debug!("[Redis] Set torrent {info_hash} counts={counts:?}");
        Ok(())
    }

    async fn get_torrent_peers(
        &self,
        info_hash: &InfoHash,
    ) -> Result<Option<TorrentPeerCounts>, CacheError> {
        let mut conn = self.conn().await?;
        let key = self.torrent_key(info_hash);
        if self.split_peers {
            let (bt_s4, bt_s6, rtc_s, bt_p4, bt_p6, rtc_p, c): (
                Option<u64>, Option<u64>, Option<u64>,
                Option<u64>, Option<u64>, Option<u64>,
                Option<u64>,
            ) = redis::cmd("HMGET")
                .arg(&key)
                .arg("bt_seeds_ipv4")
                .arg("bt_seeds_ipv6")
                .arg("rtc_seeds")
                .arg("bt_peers_ipv4")
                .arg("bt_peers_ipv6")
                .arg("rtc_peers")
                .arg("c")
                .query_async(&mut conn)
                .await
                .map_err(CacheError::RedisError)?;
            if bt_s4.is_none() && bt_s6.is_none() && rtc_s.is_none()
                && bt_p4.is_none() && bt_p6.is_none() && rtc_p.is_none()
            {
                return Ok(None);
            }
            Ok(Some(TorrentPeerCounts {
                bt_seeds_ipv4: bt_s4.unwrap_or(0),
                bt_seeds_ipv6: bt_s6.unwrap_or(0),
                rtc_seeds:     rtc_s.unwrap_or(0),
                bt_peers_ipv4: bt_p4.unwrap_or(0),
                bt_peers_ipv6: bt_p6.unwrap_or(0),
                rtc_peers:     rtc_p.unwrap_or(0),
                completed:     c.unwrap_or(0),
            }))
        } else {
            let (s, p, c): (Option<u64>, Option<u64>, Option<u64>) = redis::cmd("HMGET")
                .arg(&key)
                .arg("s")
                .arg("p")
                .arg("c")
                .query_async(&mut conn)
                .await
                .map_err(CacheError::RedisError)?;
            match (s, p) {
                (Some(seeds), Some(peers)) => Ok(Some(TorrentPeerCounts {
                    bt_seeds_ipv4: seeds,
                    bt_peers_ipv4: peers,
                    completed: c.unwrap_or(0),
                    ..Default::default()
                })),
                _ => Ok(None),
            }
        }
    }

    async fn delete_torrent(&self, info_hash: &InfoHash) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        let key = self.torrent_key(info_hash);
        conn.del::<_, ()>(&key)
            .await
            .map_err(CacheError::RedisError)?;
        debug!("[Redis] Deleted torrent {info_hash}");
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
}
