use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use std::fmt;

impl fmt::Debug for CacheConnectorMemcache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheConnectorMemcache")
            .field("client", &"<memcache::Client>")
            .field("prefix", &self.prefix)
            .finish()
    }
}