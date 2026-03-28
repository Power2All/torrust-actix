use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_redis::CacheConnectorRedis;
use crate::tracker::structs::info_hash::InfoHash;

impl CacheConnectorRedis {
    /// Build a Redis connector.  No socket is opened here — connections are
    /// created on demand inside each cache operation and dropped afterwards.
    pub async fn connect(url: &str, prefix: &str, split_peers: bool) -> Result<Self, CacheError> {
        let client = redis::Client::open(url)
            .map_err(|e| CacheError::ConnectionError(format!("Failed to create Redis client: {e}")))?;
        Ok(Self {
            client,
            prefix: prefix.to_string(),
            split_peers,
        })
    }

    pub(crate) fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }
}
