use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::tracker::structs::info_hash::InfoHash;
use parking_lot::Mutex;
use std::sync::Arc;

impl CacheConnectorMemcache {
    pub fn connect(url: &str, prefix: &str) -> Result<Self, CacheError> {
        let client = memcache::connect(url)
            .map_err(|e| CacheError::ConnectionError(format!("Failed to connect to Memcache: {e}")))?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            prefix: prefix.to_string(),
        })
    }

    pub(crate) fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }

    pub(crate) fn serialize_peers(seeds: u64, peers: u64) -> String {
        format!("{seeds}:{peers}")
    }

    pub(crate) fn deserialize_peers(value: &str) -> Option<(u64, u64)> {
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