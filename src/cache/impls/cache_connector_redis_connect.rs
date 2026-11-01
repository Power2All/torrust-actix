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

    /// Builds the cache key for a torrent's peer counts (`<prefix>t:<info_hash>`).
    pub(crate) fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }

    /// Builds the cache key for a user's totals (`<prefix>u:<id>`, `id` being the uuid or numeric id).
    pub(crate) fn user_key(&self, id: &str) -> String {
        format!("{}u:{}", self.prefix, id)
    }
}
