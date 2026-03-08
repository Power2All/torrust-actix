use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::tracker::structs::info_hash::InfoHash;

impl CacheConnectorRedis {
    pub async fn connect(url: &str, prefix: &str) -> Result<Self, CacheError> {
        let client = redis::Client::open(url)
            .map_err(|e| CacheError::ConnectionError(format!("Failed to create Redis client: {e}")))?;
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::ConnectionError(format!("Failed to connect to Redis: {e}")))?;
        Ok(Self {
            connection,
            prefix: prefix.to_string(),
        })
    }

    pub(crate) fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }
}